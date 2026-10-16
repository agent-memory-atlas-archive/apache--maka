/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';
import { resolveStorageRoot, tryAcquireInteractiveRootOwner } from '@maka/storage/root-authority';
import { openInteractiveExecutionStoresForWrite } from '@maka/storage/execution-stores';
import {
  FakeBackend,
  FAKE_ASK_USER_QUESTION_PROMPT,
  FAKE_ASK_SANDBOX_BOUNDARY_PROMPT,
  FAKE_WAIT_FOR_STEERING_PROMPT,
  FAKE_ERROR_PROMPT_PREFIX,
} from '@maka/runtime/test-only/fake-backend';
import { startExecutionRuntimeHostService } from '@maka/runtime-host/server';
import { runMakaSessionCommand } from '../run-session-command.js';
import { RunSessionInterrupted } from '../run-session-result.js';
import { randomUUID } from 'node:crypto';
import { connectRuntimeHost } from '@maka/runtime-host/client';
import { RUNTIME_HOST_PROTOCOL_VERSION } from '@maka/runtime-host/protocol';

const children = new Set<ChildProcess>();

const fixturePath = fileURLToPath(new URL('./run-session-process-fixture.js', import.meta.url));

test('real Host keeps AskUser alive across CLI processes and returns the original Turn result', {
  timeout: 120_000,
}, async () => {
  const root = await mkdtemp(join(tmpdir(), 'maka-run-session-'));
  const capability = await resolveStorageRoot({ path: root, kind: 'interactive' });
  const owner = await tryAcquireInteractiveRootOwner(capability);
  assert.ok(owner);
  const stores = await openInteractiveExecutionStoresForWrite(owner.lease);
  let sessionId: string;
  try {
    sessionId = (
      await stores.sessionStore.create({
        cwd: root,
        llmConnectionId: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa',
        llmConnectionSlug: 'fake',
        model: 'fake-model',
        permissionMode: 'ask',
      })
    ).id;
  } finally {
    await stores.sessionStore.close?.();
    await owner.close();
  }
  const { createExecutionRuntimeHostComposition } = await import(
    new URL('./execution-composition.js', import.meta.resolve('@maka/runtime-host/server')).href
  );
  const host = await startExecutionRuntimeHostService(
    { rootPath: root },
    {
      createComposition: (context, options) =>
        createExecutionRuntimeHostComposition(
          context,
          { ...options, bootstrapRuntimePolicy: false },
          {
            primaryBackendFactory: (backendContext: ConstructorParameters<typeof FakeBackend>[0]) =>
              new FakeBackend(backendContext),
            generateSessionTitle: async () => undefined,
          },
        ),
    },
  );
  try {
    const asked = await cli(root, '--session', sessionId, FAKE_ASK_USER_QUESTION_PROMPT, '--json');
    assert.equal(asked.code, 0, asked.stderr);
    const question = JSON.parse(asked.stdout);
    assert.equal(question.status, 'needs_input');
    assert.ok(question.questions.length > 0);
    const connected = await connectRuntimeHost({
      rootPath: root,
      protocol: { min: RUNTIME_HOST_PROTOCOL_VERSION, max: RUNTIME_HOST_PROTOCOL_VERSION },
    });
    assert.equal(connected.kind, 'connected');
    const probe = connected.connection;
    const subscription = await probe.openSessionSubscription({
      sessionId,
      transcript: { kind: 'none' },
    });
    await subscription.ready();
    const original = subscription.snapshot.rootTurn!;
    assert.ok(original.status === 'running' || original.status === 'waiting_for_user');
    const answers = question.questions.map(
      (q: { options: Array<{ label: string }> }) => q.options[0]!.label,
    );
    const status = await cli(root, '--session', sessionId, '--status');
    assert.equal(JSON.parse(status.stdout).status, 'needs_input');
    const answered = await cli(
      root,
      '--session',
      sessionId,
      '--answer',
      JSON.stringify(answers),
      '--json',
    );
    assert.equal(answered.code, 0, answered.stderr);
    assert.equal(JSON.parse(answered.stdout).status, 'completed');
    assert.match(JSON.parse(answered.stdout).finalOutput, /Fake question answers:/);
    const terminal = await probe.request('turn.query', { sessionId, turnId: original.turnId });
    assert.equal(terminal.status, 'completed');
    assert.equal(terminal.runId, original.runId);
    const result = await cli(root, '--session', sessionId, '--result', '--json');
    assert.equal(result.code, 0, result.stderr);
    assert.deepEqual(JSON.parse(result.stdout), JSON.parse(answered.stdout));
    const duplicate = await cli(root, '--session', sessionId, '--answer', JSON.stringify(answers));
    assert.equal(duplicate.code, 1);
    assert.match(duplicate.stderr, /found 0/);
    // Rejected commands must not elevate the saved Session permissions.
    const originalBoundary = await probe.request('session.execution_boundary.query', { sessionId });
    const invalidAnswer = await cli(
      root,
      '--session',
      sessionId,
      '--answer',
      JSON.stringify(answers),
      '--yolo',
    );
    assert.equal(invalidAnswer.code, 1);
    assert.deepEqual(
      await probe.request('session.execution_boundary.query', { sessionId }),
      originalBoundary,
    );
    // Ctrl+C while a read is in flight must not admit a new message afterward.
    let interrupt: (() => void) | undefined;
    let interruptedAdmissions = 0;
    const interruptedConnection = new Proxy(probe, {
      get(target, key) {
        if (key === 'request')
          return async (...args: Parameters<typeof probe.request>) => {
            const value = await Reflect.apply(probe.request, probe, args);
            if (args[0] === 'session.execution_boundary.query') interrupt?.();
            if (args[0] === 'turn.message.submit') interruptedAdmissions++;
            return value;
          };
        const value = Reflect.get(target, key, target);
        return typeof value === 'function' ? value.bind(target) : value;
      },
    });
    await assert.rejects(
      () =>
        runMakaSessionCommand(
          interruptedConnection,
          {
            sessionId,
            stdinPrompt: false,
            prompt: 'must-not-be-admitted',
            noWait: true,
          },
          {
            onSigint: (handler) => {
              interrupt = handler;
              return () => {
                interrupt = undefined;
              };
            },
            newId: randomUUID,
          },
        ),
      RunSessionInterrupted,
    );
    assert.equal(interruptedAdmissions, 0);
    // Keep the next Turn parked while two independent CLI messages are admitted.
    const askedAgain = await cli(
      root,
      '--session',
      sessionId,
      FAKE_ASK_USER_QUESTION_PROMPT,
      '--json',
    );
    assert.equal(askedAgain.code, 0, askedAgain.stderr);
    const wrongAnswerCount = await cli(
      root,
      '--session',
      sessionId,
      '--answer',
      JSON.stringify([...answers, 'extra']),
      '--yolo',
    );
    assert.equal(wrongAnswerCount.code, 1);
    assert.deepEqual(
      await probe.request('session.execution_boundary.query', { sessionId }),
      originalBoundary,
    );
    const activeStepCap = await cli(
      root,
      '--session',
      sessionId,
      'invalid-step-cap',
      '--max-steps',
      '1',
      '--yolo',
    );
    assert.equal(activeStepCap.code, 1);
    assert.deepEqual(
      await probe.request('session.execution_boundary.query', { sessionId }),
      originalBoundary,
    );
    const invalidResume = await cli(root, '--session', sessionId, '--resume', '--yolo');
    assert.equal(invalidResume.code, 1);
    assert.deepEqual(
      await probe.request('session.execution_boundary.query', { sessionId }),
      originalBoundary,
    );
    const queuedFirst = await cli(root, '--session', sessionId, 'queued-first', '--no-wait');
    assert.equal(queuedFirst.code, 0, queuedFirst.stderr);
    const queuedSecond = cli(root, '--session', sessionId, 'queued-second', '--wait', '--json');
    let admitted = false;
    for (let attempt = 0; attempt < 10; attempt++) {
      const current = await cli(root, '--session', sessionId, '--status');
      if (JSON.parse(current.stdout).queued >= 2) {
        admitted = true;
        break;
      }
    }
    assert.equal(admitted, true, 'both explicit follow-ups must be queued before answering');
    const waitingCurrent = await cli(root, '--session', sessionId, '--wait');
    assert.equal(JSON.parse(waitingCurrent.stdout).status, 'needs_input');
    const answeredAgain = await cli(
      root,
      '--session',
      sessionId,
      '--answer',
      JSON.stringify(answers),
      '--json',
    );
    assert.equal(answeredAgain.code, 0, answeredAgain.stderr);
    assert.match(JSON.parse(answeredAgain.stdout).finalOutput, /Fake question answers:/);
    const secondResult = await queuedSecond;
    assert.equal(secondResult.code, 0, secondResult.stderr);
    assert.match(JSON.parse(secondResult.stdout).finalOutput, /queued-second/);
    const turns = await probe.request('session.turns.query', {
      sessionId,
      throughSequence: null,
      position: 0,
      maxContributions: 128,
    });
    const first = turns.contributions.find((t) => t.userPromptPreview === 'queued-first');
    const second = turns.contributions.find((t) => t.userPromptPreview === 'queued-second');
    assert.ok(first);
    assert.ok(second);
    assert.notEqual(first.turnId, second.turnId);
    const awaitingSteer = await cli(
      root,
      '--session',
      sessionId,
      FAKE_WAIT_FOR_STEERING_PROMPT,
      '--no-wait',
    );
    assert.equal(awaitingSteer.code, 0, awaitingSteer.stderr);
    const steered = await cli(
      root,
      '--session',
      sessionId,
      'steer-this-turn',
      '--steer',
      '--wait',
      '--json',
    );
    assert.equal(steered.code, 0, steered.stderr);
    assert.match(JSON.parse(steered.stdout).finalOutput, /Acknowledged steering: steer-this-turn/);
    const failed = await cli(
      root,
      '--session',
      sessionId,
      `${FAKE_ERROR_PROMPT_PREFIX}rate_limit`,
      '--json',
    );
    assert.equal(failed.code, 1);
    assert.match(JSON.parse(failed.stdout).error, /Deterministic rate_limit failure/);
    const failedResult = await cli(root, '--session', sessionId, '--result', '--json');
    assert.equal(failedResult.code, 1);
    assert.match(JSON.parse(failedResult.stdout).error, /Deterministic rate_limit failure/);
    const failedStatus = await cli(root, '--session', sessionId, '--status', '--json');
    assert.equal(failedStatus.code, 0, failedStatus.stderr);
    assert.equal(JSON.parse(failedStatus.stdout).status, 'failed');
    // A detached non-bypass Turn can request a boundary after its launcher exits.
    const detachedBoundary = await cli(
      root,
      '--session',
      sessionId,
      FAKE_ASK_SANDBOX_BOUNDARY_PROMPT,
      '--no-wait',
    );
    assert.equal(detachedBoundary.code, 0, detachedBoundary.stderr);
    const blocked = await cli(root, '--session', sessionId, '--wait', '--json');
    assert.equal(blocked.code, 1);
    assert.match(blocked.stderr, /Pending sandbox_boundary/);
    assert.match(blocked.stderr, /--stop/);
    assert.doesNotMatch(blocked.stderr, /MCP form/);
    const blockedSubscription = await probe.openSessionSubscription({
      sessionId,
      transcript: { kind: 'none' },
    });
    await blockedSubscription.ready();
    const blockedTurn = blockedSubscription.snapshot.rootTurn!;
    assert.ok(
      blockedSubscription.snapshot.interactions.pending.some(
        (p) => p.request.kind === 'sandbox_boundary',
      ),
    );
    await blockedSubscription.close();
    const stopped = await cli(root, '--session', sessionId, '--stop', '--json');
    assert.equal(stopped.code, 0, stopped.stderr);
    const cancelled = await probe.request('turn.query', { sessionId, turnId: blockedTurn.turnId });
    assert.equal(cancelled.status, 'cancelled');
    const afterStop = await cli(
      root,
      '--session',
      sessionId,
      'after-explicit-stop',
      '--wait',
      '--json',
    );
    assert.equal(afterStop.code, 0, afterStop.stderr);
    assert.match(JSON.parse(afterStop.stdout).finalOutput, /after-explicit-stop/);
    await subscription.close();
    await probe.close();
  } finally {
    for (const child of children) child.kill('SIGTERM');
    await host.close();
    await rm(root, { recursive: true, force: true });
  }
});

async function cli(
  root: string,
  ...args: string[]
): Promise<{ code: number | null; stdout: string; stderr: string }> {
  const child = spawn(process.execPath, [fixturePath, root, ...args], {
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  children.add(child);
  let stdout = '';
  let stderr = '';
  child.stdout!.setEncoding('utf8').on('data', (data) => {
    stdout += data;
  });
  child.stderr!.setEncoding('utf8').on('data', (data) => {
    stderr += data;
  });
  return await new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('close', (code) => {
      children.delete(child);
      resolve({ code, stdout, stderr });
    });
  });
}
