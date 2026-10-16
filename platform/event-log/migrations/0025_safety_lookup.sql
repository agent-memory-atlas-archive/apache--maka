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

-- Skip observation payloads when checking execution boundaries and effects.
-- OR terms also let single-kind queries prove they match this partial index.
CREATE INDEX safety_session_kind_sequence ON event_log(
    json_extract(event_json, '$.invocation.session_id'), kind, sequence
) WHERE kind = 'invocation_opened' OR kind = 'tool_dispatched'
    OR kind = 'model_requested' OR kind = 'model_completed'
    OR kind = 'model_interrupted';
