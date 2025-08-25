import { invoke } from '@tauri-apps/api/core'

export interface JsonToGoOptions {
  struct_name: string
  exported_fields: boolean
  is_go_118_or_above: boolean
  is_go_124_or_above: boolean
  selected_tags: Record<string, boolean>
}

export interface DevToolResponse<T> {
  data?: T
  success: boolean
  error?: string
}


export async function convertJsonToGo(
  json: string,
  options: JsonToGoOptions
): Promise<DevToolResponse<string>> {
  try {
    const response = await invoke<DevToolResponse<string>>('convert_json_to_go', {
      jsonStr: json,
      options,
    })
    return response
  } catch (error) {
    console.error('API call failed:', error)
    return {
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    }
  }
}