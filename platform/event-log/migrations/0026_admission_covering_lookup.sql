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

-- runtime_events excludes collected bodies. Put that predicate in the indexes
-- so proving every old opening is sealed never loads its event_json payload.
CREATE INDEX admission_openings ON event_log(
    json_extract(event_json, '$.invocation.session_id'), invocation_id
) WHERE kind = 'invocation_opened' AND event_json IS NOT NULL;

-- Uniqueness already follows from invocation_boundary. Retaining that planner
-- fact makes SQLite choose this covering subset over the older boundary index.
CREATE UNIQUE INDEX admission_terminals ON event_log(invocation_id, kind)
WHERE kind = 'invocation_ended' AND event_json IS NOT NULL;
