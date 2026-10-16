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

macro_rules! summary_format {
    () => { "## Goal\n[What the user is trying to accomplish]\n\n## Progress\n### Done\n- [Completed work and changes]\n### In Progress\n- [Current work]\n\n## Key Decisions\n- **[Decision]**: [Brief rationale]\n\n## Next Steps\n1. [Ordered list of what should happen next]\n\n## Critical Context\n- [Files, commands/results, errors, anything needed to continue; or \"(none)\"]" };
}

pub const SUMMARY_FORMAT_TEMPLATE: &str = summary_format!();
pub const DEFAULT_COMPACTION_PROMPT: &str = concat!(
    "You are a context summarization assistant.\nRead the conversation between a user and an AI assistant, then produce a structured summary another LLM will use to continue the same task.\nDo NOT continue the conversation. Do NOT answer questions in it. ONLY output the structured summary.\n\nUse this exact format:\n\n",
    summary_format!(),
    "\n\nKeep each section concise. Preserve exact file paths, function names, commands, and error messages.\n\nNow write the structured summary of the conversation above. Output only the summary.\n\nThe complete original conversation remains available through Recall when details are needed."
);
