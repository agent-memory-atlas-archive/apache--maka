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

-- Native local UI snapshots retain their original namespace and CAS revision.
CREATE TABLE application_ui_checkpoints (
    deployment TEXT NOT NULL,
    instance TEXT NOT NULL CHECK(json_valid(instance)),
    key TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK(revision > 0),
    format TEXT NOT NULL,
    data_source_json TEXT NOT NULL CHECK(json_valid(data_source_json)),
    metadata_json TEXT NOT NULL CHECK(json_valid(metadata_json)),
    content BLOB NOT NULL,
    PRIMARY KEY(deployment, instance, key)
) WITHOUT ROWID;
