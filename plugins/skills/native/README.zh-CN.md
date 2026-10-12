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

# maka-skills-native

[English](README.md)

将技能库作为普通 Host 服务提供。`provide` 注册工厂，发现与解析委托 `maka-skills`。服务管理有界读取、目录修订和发布回执。调用方使用发现结果中的精确引用与修订，并原样传回分页续读对象。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
