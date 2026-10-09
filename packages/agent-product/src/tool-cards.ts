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

import type { Message } from './contracts.js';
import { sameValue } from './value.js';

/** Merge only reciprocal Host references, retaining both native source bodies.
 * A counterpart outside this bounded window remains accessible by its reference.
 */
export function toolCards(
  messages: readonly Message[],
): { message: Message; result: Message | null }[] {
  const bySequence = new Map<number, Message[]>();
  for (const message of messages) {
    if (message.source !== null && message.tool) {
      const rows = bySequence.get(message.source) ?? [];
      rows.push(message);
      bySequence.set(message.source, rows);
    }
  }
  const resultsByCall = new Map<number, Set<number>>();
  for (const message of messages) {
    if (message.role === 'tool_result' && message.source !== null && message.tool?.call) {
      const sequences = resultsByCall.get(message.tool.call.sequence) ?? new Set<number>();
      sequences.add(message.source);
      resultsByCall.set(message.tool.call.sequence, sequences);
    }
  }
  const paired = new Map<Message, Message>();
  const consumed = new Set<Message>();
  for (const call of messages) {
    const metadata = call.tool;
    if (
      call.role !== 'tool_call' ||
      (resultsByCall.get(call.source!)?.size ?? 0) > 1 ||
      !metadata?.call ||
      !metadata.result ||
      metadata.call.sequence !== call.source ||
      bySequence.get(call.source)?.length !== 1
    )
      continue;
    const candidates = bySequence.get(metadata.result.sequence);
    if (candidates?.length !== 1) continue;
    const result = candidates[0]!;
    if (
      result.role !== 'tool_result' ||
      result.source! <= call.source ||
      result.tool?.toolUseId !== metadata.toolUseId ||
      result.tool.call?.sequence !== call.source ||
      !sameValue(result.tool.call, metadata.call) ||
      result.tool.result?.sequence !== result.source ||
      result.tool.result.isError !== metadata.result.isError ||
      result.tool.result.image !== metadata.result.image
    )
      continue;
    paired.set(call, result);
    consumed.add(result);
  }
  return messages
    .filter((message) => !consumed.has(message))
    .map((message) => ({
      message:
        message.tool?.call &&
        (message.role === 'tool_call' ||
          (!bySequence.has(message.tool.call.sequence) &&
            (resultsByCall.get(message.tool.call.sequence)?.size ?? 0) === 1))
          ? { ...message, id: message.tool.toolUseId }
          : message,
      result: paired.get(message) ?? null,
    }));
}
