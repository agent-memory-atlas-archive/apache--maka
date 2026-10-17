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

-- Only diagnostic candidates, never model body chunks, participate in this lookup.
CREATE INDEX diagnostic_session_sequence ON event_log(
    CASE WHEN kind='title_failure' THEN json_extract(event_json,'$.opening.session_id') ELSE json_extract(event_json,'$.invocation.session_id') END, sequence
) WHERE (kind='title_failure' OR (kind='invocation_ended' AND json_extract(event_json,'$.fact.outcome.kind')='failed') OR (kind='model_interrupted' AND json_extract(event_json,'$.fact.status')!='cancelled'));
