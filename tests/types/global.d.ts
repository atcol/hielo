import { Browser } from '@playwright/test';
import { ChildProcess } from 'child_process';

declare global {
  var __HIELO_PROCESS__: ChildProcess | undefined;
  var __HIELO_BROWSER__: Browser | undefined;
}

export {};