import http from './index'

export interface VideoCookieStatus {
  platform: string
  cookie_count: number
  domains: string[]
  updated_at: string
}

export interface VideoCookieBackup {
  version: number
  platform: string
  encrypted_payload: string
  cookie_count: number
  domains: string[]
  updated_at: string
}

interface CookieListResponse {
  platforms: string[]
  cookies: VideoCookieStatus[]
}

export const videoCookieApi = {
  list() {
    return http.get<unknown, { code: number; msg: string; data: CookieListResponse }>(
      '/admin/video-cookies',
    )
  },

  importJson(platform: string, cookies: unknown) {
    return http.post<unknown, { code: number; msg: string; data: VideoCookieStatus }>(
      '/admin/video-cookies/import',
      { platform, cookies },
    )
  },

  exportBackup(platform: string) {
    return http.get<unknown, { code: number; msg: string; data: VideoCookieBackup }>(
      `/admin/video-cookies/${encodeURIComponent(platform)}/export`,
    )
  },

  importBackup(backup: VideoCookieBackup) {
    return http.post<unknown, { code: number; msg: string; data: VideoCookieStatus }>(
      '/admin/video-cookies/import-backup',
      backup,
    )
  },

  remove(platform: string) {
    return http.delete<unknown, { code: number; msg: string }>(
      `/admin/video-cookies/${encodeURIComponent(platform)}`,
    )
  },
}
