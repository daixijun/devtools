import { invoke } from '@tauri-apps/api/core'

export interface JsonToGoOptions {
  struct_name: string
  exported_fields: boolean
  is_go_118_or_above: boolean
  is_go_124_or_above: boolean
  selected_tags: Record<string, boolean>
}

export async function convertJsonToGo(
  json: string,
  options: JsonToGoOptions,
): Promise<string> {
  return await invoke<string>('convert_json_to_go', {
    jsonStr: json,
    options,
  })
}
