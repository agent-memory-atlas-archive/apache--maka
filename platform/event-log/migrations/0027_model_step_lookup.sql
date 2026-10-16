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

-- Keep the existing catalog paths while allowing single-kind lookups to use
-- the same partial index, including an absent executor in a long model run.
DROP INDEX catalog_model_boundaries;
CREATE INDEX catalog_model_boundaries ON event_log(invocation_id, kind, sequence)
WHERE kind = 'model_requested' OR kind = 'model_completed'
   OR kind = 'model_interrupted' OR kind = 'executor_started';

-- Resolve each request's terminal without scanning the containing invocation.
-- The body predicate preserves runtime_events' exclusion of collected records.
CREATE INDEX model_step_terminals ON event_log(invocation_id, operation_id, sequence)
WHERE kind IN ('model_completed', 'model_interrupted') AND event_json IS NOT NULL;
