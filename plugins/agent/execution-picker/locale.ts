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

import { createCopy } from '../../shared/locale.js';
import {
  msg_check,
  msg_closeModels,
  msg_executionAuthenticate,
  msg_executionAuthenticated,
  msg_executionBackend,
  msg_executionCancel,
  msg_executionCancelling,
  msg_executionCheck,
  msg_executionChecking,
  msg_executionCopyUrl,
  msg_executionDefault,
  msg_executionModel,
  msg_executionModelHint,
  msg_executionModelName,
  msg_executionNoAuth,
  msg_executionReceipt,
  msg_executionSettingsFailed,
  msg_executionUnavailable,
  msg_executionUnresolved,
  msg_executionUnsupported,
  msg_modelApply,
  msg_models,
  msg_refreshModels,
  msg_thinking,
  msg_thinkingHigh,
  msg_thinkingLow,
  msg_thinkingMax,
  msg_thinkingMedium,
  msg_thinkingMinimal,
  msg_thinkingOff,
  msg_thinkingUltra,
  msg_thinkingXhigh,
} from '../../shared/locale-messages.js';
import { translate } from '../../shared/tui-locale.js';
import {
  tui_settings_advanced,
  tui_settings_appearance,
  tui_settings_assistant,
  tui_settings_choose,
  tui_settings_host,
  tui_settings_host_connected,
  tui_settings_host_connecting,
  tui_settings_host_details,
  tui_settings_host_disconnected,
  tui_settings_host_failed,
  tui_settings_insights,
  tui_settings_interface,
  tui_settings_jev,
  tui_settings_language,
  tui_settings_models,
  tui_settings_motion,
  tui_settings_motion_off,
  tui_settings_motion_on,
  tui_settings_palette,
  tui_settings_palette_custom_fallback,
  tui_settings_palette_dark,
  tui_settings_palette_default,
  tui_settings_palette_dusk,
  tui_settings_palette_paper,
  tui_settings_sessions,
  tui_settings_subagents,
  tui_settings_symbols,
  tui_settings_symbols_ascii,
  tui_settings_symbols_unicode,
  tui_settings_web,
  tui_thinking_default,
  tui_thinking_high,
  tui_thinking_low,
  tui_thinking_max,
  tui_thinking_medium,
  tui_thinking_minimal,
  tui_thinking_off,
  tui_thinking_steps,
  tui_thinking_ultra,
  tui_thinking_xhigh,
} from '../../shared/tui-messages.js';

// This feature owns this explicit message catalog; shared messages remain tree-shakeable.

const messages = {
  check: msg_check,
  closeModels: msg_closeModels,
  executionAuthenticate: msg_executionAuthenticate,
  executionAuthenticated: msg_executionAuthenticated,
  executionBackend: msg_executionBackend,
  executionCancel: msg_executionCancel,
  executionCancelling: msg_executionCancelling,
  executionCheck: msg_executionCheck,
  executionChecking: msg_executionChecking,
  executionCopyUrl: msg_executionCopyUrl,
  executionDefault: msg_executionDefault,
  executionModel: msg_executionModel,
  executionModelHint: msg_executionModelHint,
  executionModelName: msg_executionModelName,
  executionNoAuth: msg_executionNoAuth,
  executionReceipt: msg_executionReceipt,
  executionSettingsFailed: msg_executionSettingsFailed,
  executionUnavailable: msg_executionUnavailable,
  executionUnresolved: msg_executionUnresolved,
  executionUnsupported: msg_executionUnsupported,
  modelApply: msg_modelApply,
  models: msg_models,
  refreshModels: msg_refreshModels,
  thinking: msg_thinking,
  thinkingHigh: msg_thinkingHigh,
  thinkingLow: msg_thinkingLow,
  thinkingMax: msg_thinkingMax,
  thinkingMedium: msg_thinkingMedium,
  thinkingMinimal: msg_thinkingMinimal,
  thinkingOff: msg_thinkingOff,
  thinkingUltra: msg_thinkingUltra,
  thinkingXhigh: msg_thinkingXhigh,
};
export const copy = /* @__PURE__ */ createCopy(messages);
export type Copy = ReturnType<typeof copy>;

const terminalMessages = {
  'settings-advanced': tui_settings_advanced,
  'settings-appearance': tui_settings_appearance,
  'settings-assistant': tui_settings_assistant,
  'settings-choose': tui_settings_choose,
  'settings-host': tui_settings_host,
  'settings-host-connected': tui_settings_host_connected,
  'settings-host-connecting': tui_settings_host_connecting,
  'settings-host-details': tui_settings_host_details,
  'settings-host-disconnected': tui_settings_host_disconnected,
  'settings-host-failed': tui_settings_host_failed,
  'settings-insights': tui_settings_insights,
  'settings-interface': tui_settings_interface,
  'settings-jev': tui_settings_jev,
  'settings-language': tui_settings_language,
  'settings-models': tui_settings_models,
  'settings-motion': tui_settings_motion,
  'settings-motion-off': tui_settings_motion_off,
  'settings-motion-on': tui_settings_motion_on,
  'settings-palette': tui_settings_palette,
  'settings-palette-custom-fallback': tui_settings_palette_custom_fallback,
  'settings-palette-dark': tui_settings_palette_dark,
  'settings-palette-default': tui_settings_palette_default,
  'settings-palette-dusk': tui_settings_palette_dusk,
  'settings-palette-paper': tui_settings_palette_paper,
  'settings-sessions': tui_settings_sessions,
  'settings-subagents': tui_settings_subagents,
  'settings-symbols': tui_settings_symbols,
  'settings-symbols-ascii': tui_settings_symbols_ascii,
  'settings-symbols-unicode': tui_settings_symbols_unicode,
  'settings-web': tui_settings_web,
  'thinking-default': tui_thinking_default,
  'thinking-high': tui_thinking_high,
  'thinking-low': tui_thinking_low,
  'thinking-max': tui_thinking_max,
  'thinking-medium': tui_thinking_medium,
  'thinking-minimal': tui_thinking_minimal,
  'thinking-off': tui_thinking_off,
  'thinking-steps': tui_thinking_steps,
  'thinking-ultra': tui_thinking_ultra,
  'thinking-xhigh': tui_thinking_xhigh,
};
export function tuiText(
  locale: string,
  key: string,
  values: Record<string, string | number> = {},
): string {
  return translate(locale, terminalMessages, key, values);
}
