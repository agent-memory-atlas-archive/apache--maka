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

import type {
  ActionOutcome,
  Component,
  ComponentInput,
  HostRequest,
  RequestContext,
  Surface,
} from '@maka-agent/plugin-sdk';

function originalUiReceipt(outcome: ActionOutcome) {
  if (outcome.status !== 'operation') return;
  const original = outcome.reply.reference;
  // @ts-expect-error Unavailable/not-recorded receipts do not describe an outcome.
  const premature = outcome.reply.receipt.outcome;
  if (outcome.reply.receipt.kind === 'recorded') {
    const known = outcome.reply.receipt.outcome;
    void known;
  }
  void [original, premature];
}
void originalUiReceipt;

const terminal = { kind: 'terminal', binding: 'surface' } satisfies Surface;
// @ts-expect-error Terminal surfaces have no pixel width.
const badTerminal = { kind: 'terminal', binding: 'surface', width: 400 } satisfies Surface;
// @ts-expect-error Desktop surfaces must supply a complete window recipe.
const badWindow = { kind: 'window', binding: 'surface' } satisfies Surface;
// @ts-expect-error Component notifications carry the exact child and event name.
const incompleteEvent = { kind: 'component', key: 'changed', event: {} } satisfies ComponentInput;

const callbacks: Component = {
  // @ts-expect-error Foreground callbacks cannot return promises.
  async mount() {
    return {
      frame: {
        revision: 0,
        state: {},
        surface: terminal,
        view: { kind: 'button', key: 'ok', label: 'OK' },
      },
      events: [],
      actions: [],
    };
  },
  update() {
    throw Error('unused');
  },
  event() {
    throw Error('unused');
  },
};
const fabricated = {
  operation: 'read',
  slot: 'text',
  start: 0,
  end: 1,
  // @ts-expect-error Resource slots cannot carry fabricated bearer handles.
  token: 'grant',
} satisfies HostRequest;
const incompleteCall = {
  operation: 'call',
  import: 'echo',
  provider: 0,
  method: 'echo',
  input: null,
} as const;
// @ts-expect-error Every forward maps actual resource slots explicitly.
const missingSlots: HostRequest = incompleteCall;
async function unknownResult(context: RequestContext) {
  const result = await context.invoke({
    operation: 'call',
    import: 'echo',
    provider: 0,
    method: 'echo',
    input: null,
    attachments: {},
  });
  // @ts-expect-error An unchecked service result is not consumer-selected T.
  const text: string = result;
}
void [
  terminal,
  badTerminal,
  badWindow,
  incompleteEvent,
  callbacks,
  fabricated,
  incompleteCall,
  missingSlots,
  unknownResult,
];

const mutation = {
  operation: 'mutate',
  import: 'echo',
  provider: 0,
  method: 'echo',
  input: null,
  operationId: '00000000-0000-4000-8000-000000000001',
} satisfies HostRequest;
const incompleteMutation = {
  operation: 'mutate',
  import: 'echo',
  provider: 0,
  method: 'echo',
  input: null,
} as const;
// @ts-expect-error An explicit durable operation identity is required.
const missingOperation: HostRequest = incompleteMutation;
const fabricatedMutation = {
  ...mutation,
  // @ts-expect-error Live resource attachments cannot masquerade as durable snapshots.
  attachments: { text: 'text' },
} satisfies HostRequest;
void [mutation, missingOperation, fabricatedMutation];

async function durableReceipt(context: RequestContext) {
  const reply = await context.invoke(mutation);
  const operation: string = reply.reference.operation;
  // @ts-expect-error Acceptance never proves that the journal has a result.
  const prematureOutcome = reply.receipt.outcome;
  if (reply.receipt.kind === 'recorded' && reply.receipt.outcome.kind === 'returned') {
    // @ts-expect-error Business output must still be checked against its contract.
    const text: string = reply.receipt.outcome.value;
    void text;
  }
  const recovered = await context.invoke({ operation: 'recover', reference: reply.reference });
  const sameSource = recovered.reference.source;
  if (recovered.receipt.kind === 'unavailable') {
    const reason: string = recovered.receipt.diagnostic;
    void reason;
  }
  void [operation, prematureOutcome, sameSource];
}
const incompleteRecovery = { operation: 'recover' } as const;
// @ts-expect-error Recovery requires the original reference, not a current provider alias.
const missingReference: HostRequest = incompleteRecovery;
void [durableReceipt, missingReference];
