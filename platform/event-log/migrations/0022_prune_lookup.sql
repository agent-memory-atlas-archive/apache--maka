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

-- Pruning only needs settled tool results, even in a long text conversation.
CREATE INDEX tool_settled_session_sequence ON event_log(
    json_extract(event_json, '$.invocation.session_id'), sequence
) WHERE kind = 'tool_settled';

-- A tool proof belongs to one model step, not the entire long-running invocation.
CREATE INDEX model_observed_step ON event_log(
    invocation_id, CAST(json_extract(event_json, '$.fact.step_id') AS TEXT), sequence
) WHERE kind = 'model_observed';
