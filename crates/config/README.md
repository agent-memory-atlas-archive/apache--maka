<!--
  Licensed to the Apache Software Foundation (ASF) under one
  or more contributor license agreements.  See the NOTICE file
  distributed with this work for additional information
  regarding copyright ownership.  The ASF licenses this file
  to you under the Apache License, Version 2.0 (the
  "License"); you may not use this file except in compliance
  with the License.  You may obtain a copy of the License at

      http://www.apache.org/licenses/LICENSE-2.0

  Unless required by applicable law or agreed to in writing,
  software distributed under the License is distributed on an
  "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
  KIND, either express or implied.  See the License for the
  specific language governing permissions and limitations
  under the License.
-->

# Root configuration

`ConfigurationStore::for_root` shares the actual `RootOwner`. Its owned SQL connection handles the connection catalog, credential vault and their independent revisions. The embedding drains `shutdown` before releasing its native owner; catalog/status snapshots grant no execution or Root access.

Connection changes compare the caller's exact catalog or connection basis in the write transaction. Credential deletion requires an explicit `expectedConnection`: the original complete connection target for connection credentials, or null for network credentials. The same vault transaction checks this target before the credential generation; changing an endpoint cannot leave an old deletion request valid merely because the saved key is unchanged.

`oauth::LoginStart` requires `expectedCredential`, including explicit null when the caller observed no credential. Existing login targets contain their original `ConnectionCredentialTarget`. Preparation compares both in one SQL snapshot, and completion compares the prepared connection and vault generation again. Refresh or logout cannot be silently overwritten by a request based on an earlier observation. The original authentication fingerprint includes that observation; recovery observes the original attempt and never grants a second exchange.

Provider credentials use the authentication/enrollment owner and validated credential envelopes. Generic `set_credential` rejects provider keys. Trusted native extensions can use the same provider `Definition::authenticate` validation after their actual Host admission, then publish through the prepared enrollment. Authentication input and replacement secrets stay private; public queries return only catalog facts and credential status. Commit uncertainty remains distinct from confirmed business success and from physical cleanup.
