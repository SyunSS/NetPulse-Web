/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

/**
 * 格式化毫秒时间
 */
export function formatMs(ms: number): string {
  if (ms < 1000) return ms.toFixed(0) + ' ms'
  return (ms / 1000).toFixed(2) + ' s'
}

/**
 * 格式化时间戳（强制 Asia/Shanghai 时区，与服务端时区无关）
 */
export function formatTime(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleString('zh-CN', { timeZone: 'Asia/Shanghai' })
}

/**
 * 格式化时间戳，仅时分秒
 */
export function formatTimeShort(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleTimeString('zh-CN', { timeZone: 'Asia/Shanghai', hour12: false })
}

/**
 * 格式化日期（短）
 */
export function formatDateShort(dateStr: string): string {
  const d = new Date(dateStr)
  return d.toLocaleDateString('zh-CN', { timeZone: 'Asia/Shanghai' })
}
