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

# maka-product-native

[English](README.md)

选择编译进 Maka 产品的原生 Host 工厂。`provide` 从插件 crate 组装注册表。添加工厂只让实现变得可用；包选择与组合仍决定是否创建实例、为其提供哪些导入。业务行为应留在所属插件中。

[源码](src/lib.rs) · [Cargo 配置](Cargo.toml)
