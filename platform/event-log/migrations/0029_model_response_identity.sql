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

-- Answer identity lookups must skip text/tool observations, including steps
-- with no provider-reported model. Canonical facts remain the authority.
CREATE INDEX model_response_identity ON event_log(
    invocation_id,
    CAST(json_extract(event_json, '$.fact.step_id') AS TEXT),
    sequence
) WHERE kind = 'model_observed'
  AND event_json IS NOT NULL
  AND json_extract(event_json, '$.fact.event.kind') = 'response_metadata'
  AND json_type(event_json, '$.fact.event.data.model') = 'text';
