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

import type { PackagePresentation } from '../../packages/plugin-sdk/src/index.js';

// Keep package identity separate from author-provided, localized display copy.
export function pluginDescription(
  info: PackagePresentation | undefined,
  locale: string,
  fallback: string,
) {
  const tag = locale.toLowerCase();
  const variants = Object.entries(info?.localizations ?? {});
  const localized =
    variants.find(([key]) => key.toLowerCase() === tag)?.[1] ??
    variants.find(([key]) => tag.startsWith(key.toLowerCase() + '-'))?.[1] ??
    (tag.startsWith('zh') && !/hant|tw|hk|mo/.test(tag)
      ? variants.find(([key]) => key.toLowerCase() === 'zh-cn')?.[1]
      : undefined);
  return {
    name: localized?.displayName?.trim() || info?.displayName?.trim() || fallback,
    description: localized?.description?.trim() || info?.description?.trim() || '',
  };
}
