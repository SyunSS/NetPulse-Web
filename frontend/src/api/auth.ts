import http from './index'

export interface LoginRequest {
  username: string
  password: string
}

export interface RegisterRequest {
  username: string
  password: string
}

export interface UserInfo {
  id: string
  username: string
  role: string
}

export interface LoginResponse {
  token: string
  user: UserInfo
}

export const authApi = {
  login(data: LoginRequest) {
    return http.post<unknown, { code: number; msg: string; data: LoginResponse }>(
      '/auth/login',
      data,
    )
  },

  register(data: RegisterRequest) {
    return http.post<unknown, { code: number; msg: string; data: UserInfo }>(
      '/auth/register',
      data,
    )
  },
}
