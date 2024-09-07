import { invoke } from '@tauri-apps/api/tauri'

export async function cached(url: string): Promise<string> {
  return await invoke('plugin:remote-caching|cached', {url: url}) as unknown as string;
}

export async function clear_cache(): Promise<void | string> {
  return await invoke('plugin:remote-caching|clear_cache') as unknown as (void | string);
}
