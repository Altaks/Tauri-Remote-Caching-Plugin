import { invoke } from '@tauri-apps/api/tauri'

export async function cached(url: string): Promise<string> {
  console.log("Invoking cache system for image : " + url);
  let result = await invoke('plugin:remote-caching|cached', {url: url}) as unknown as string;
  console.log("Cache system result found : " + url);
  return result;
}
