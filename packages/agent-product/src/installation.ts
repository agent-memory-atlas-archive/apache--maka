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

import type { CompositionConfiguration } from './composition.js';

/** Trusted launcher inputs. Product defaults live with the ordinary composition. */
export interface InstallationOptions {
  workspace: string;
  hostRoot: string;
  helper: string;
  themeFile: string;
  home?: string;
  locale?: string;
  scheme?: 'light' | 'dark';
}

export function installation(
  options: InstallationOptions,
): Omit<CompositionConfiguration, 'packages' | 'keys' | 'hostDeployment' | 'managementDeployment'> {
  const workspaceRoots: CompositionConfiguration['agent']['workspaceRoots'] = [
    { label: 'Workspace', path: options.workspace },
  ];
  const tools: CompositionConfiguration['agent']['tools'] = [];
  const add = (
    service: string,
    method: string,
    name: string,
    description: string,
    result: 'json' | 'image' = 'json',
  ) =>
    tools.push({ import: service, method, name, description, result, confirmation: 'preapproved' });
  add('files', 'readText', 'Read', 'Read text from a file in the selected workspace.');
  add('files', 'readImage', 'ReadImage', 'Read an image in the selected workspace.', 'image');
  add('files', 'glob', 'Glob', 'Find files in the selected workspace.');
  add('files', 'grep', 'Grep', 'Search file contents in the selected workspace.');
  add('files', 'write', 'Write', 'Write a file in the selected workspace.');
  add('files', 'edit', 'Edit', 'Edit a file in the selected workspace.');
  add('files', 'applyPatch', 'ApplyPatch', 'Apply a patch in the selected workspace.');
  add('processes', 'execute', 'Shell', 'Run a command under the selected workspace permissions.');
  add('processes', 'read', 'ProcessRead', 'Read output from a previously started command.');
  add('processes', 'writeStdin', 'ProcessInput', 'Send input to a previously started command.');
  add('processes', 'stop', 'ProcessStop', 'Stop a previously started command.');
  add('questions', 'ask', 'AskUserQuestion', 'Ask the user for decisions needed to continue.');
  add('skills', 'search', 'SkillSearch', 'Find applicable skills in the selected collections.');
  add('skills', 'load', 'Skill', 'Read a selected skill and its resources.');
  const agent: CompositionConfiguration['agent'] = {
    workspaceRoots,
    importRoots: [{ label: 'Conversation sources', path: options.home ?? options.workspace }],
    permissions: { access: 'workspace_write', requestPermissions: true },
    maxActiveRuns: 4,
    maxSteps: 16,
    recall: true,
    tools,
    skillLibrary: 'skills',
  };
  const processes: NonNullable<CompositionConfiguration['processes']> = {
    filesystem: {
      default: 'read',
      rules: [{ path: options.workspace, scope: 'subtree', access: 'write' }],
      denyGlobs: [],
    },
    network: 'denied',
    helper: options.helper,
  };
  return {
    presentation: 'tui',
    locale: options.locale ?? 'auto',
    scheme: options.scheme ?? 'dark',
    hostRoot: options.hostRoot,
    themeFile: options.themeFile,
    agent,
    processes,
    coordinator: agent,
    coordinatorProcesses: processes,
    skills: {
      sources: [],
      bundled: true,
      manage: true,
      preferences: true,
      projectRoots: workspaceRoots,
      ...(options.home ? { home: options.home } : {}),
    },
  };
}
