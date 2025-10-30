import { invoke } from '@tauri-apps/api/core'
import React, { useCallback, useMemo, useRef, useState } from 'react'
import { ToolLayout } from '../components/layouts'

type WhoisParsed = {
  domain: string
  source: string
  registrar?: string | null
  registrant?: string | null
  created?: string | null
  expires?: string | null
  updated?: string | null
  status?: string[] | null
  nameServers?: string[] | null
  rawText?: string | null
}

type DomainResult = {
  domain: string
  best?: WhoisParsed
  channels: WhoisParsed[]
  error?: string
  startedAt?: number
  finishedAt?: number
}

type ProgressStats = {
  total: number
  completed: number
  success: number
  avgMs: number
}

// Simple per-channel rate limiter: ensures min interval between requests per channel
class RateLimiter {
  private lastTime: Map<string, number> = new Map()
  private timers: Map<string, Promise<void>> = new Map()

  constructor(private minIntervalMs: Record<string, number>) {}

  async wait(channel: string) {
    const now = Date.now()
    const minInterval = this.minIntervalMs[channel] ?? 300
    const last = this.lastTime.get(channel) ?? 0
    const elapsed = now - last
    const delay = Math.max(0, minInterval - elapsed)
    const promise = new Promise<void>((resolve) => setTimeout(resolve, delay))
    this.timers.set(channel, promise)
    await promise
    this.lastTime.set(channel, Date.now())
  }
}

// Cache with TTL using localStorage
const CACHE_KEY = 'whois_cache_v2'
const HISTORY_KEY = 'whois_history_v1'
const CACHE_TTL_MS = 24 * 60 * 60 * 1000

function loadCache(): Record<string, { ts: number; data: DomainResult }> {
  try {
    const raw = localStorage.getItem(CACHE_KEY)
    if (!raw) return {}
    return JSON.parse(raw)
  } catch {
    return {}
  }
}

function saveCache(cache: Record<string, { ts: number; data: DomainResult }>) {
  try {
    localStorage.setItem(CACHE_KEY, JSON.stringify(cache))
  } catch {}
}

function loadHistory(): DomainResult[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    if (!raw) return []
    return JSON.parse(raw)
  } catch {
    return []
  }
}

function saveHistory(history: DomainResult[]) {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history.slice(-200)))
  } catch {}
}

function exportCSV(results: DomainResult[]) {
  const headers = [
    'domain',
    'source',
    'registrar',
    'registrant',
    'created',
    'expires',
    'updated',
    'status',
    'nameServers',
  ]
  const rows = results.map((r) => {
    const b = r.best
    const status = b?.status?.join('|') ?? ''
    const ns = b?.nameServers?.join('|') ?? ''
    return [
      r.domain,
      b?.source ?? '',
      b?.registrar ?? '',
      b?.registrant ?? '',
      b?.created ?? '',
      b?.expires ?? '',
      b?.updated ?? '',
      status,
      ns,
    ]
  })
  const csv =
    headers.join(',') +
    '\n' +
    rows
      .map((cols) =>
        cols.map((c) => '"' + (c ?? '').replace(/"/g, '""') + '"').join(','),
      )
      .join('\n')
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `whois_results_${Date.now()}.csv`
  a.click()
  URL.revokeObjectURL(url)
}

function exportJSON(results: DomainResult[]) {
  const blob = new Blob([JSON.stringify(results, null, 2)], {
    type: 'application/json;charset=utf-8',
  })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = `whois_results_${Date.now()}.json`
  a.click()
  URL.revokeObjectURL(url)
}

function pickBest(parsed: WhoisParsed[] | undefined): WhoisParsed | undefined {
  if (!parsed || parsed.length === 0) return undefined
  // Prefer RDAP structured source, then WHOIS text
  const rdap = parsed.find((p) => p.source.includes('rdap'))
  if (rdap) return rdap
  return parsed[0]
}

const WhoisLookup: React.FC = () => {
  const [input, setInput] = useState('')
  const [results, setResults] = useState<Record<string, DomainResult>>({})
  const [running, setRunning] = useState(false)
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const limiter = useMemo(
    () =>
      new RateLimiter({
        whois_verisign: 700,
        whois_cndns: 700,
        whois_hichina: 700,
      }),
    [],
  )
  const cacheRef = useRef(loadCache())
  const [history, setHistory] = useState<DomainResult[]>(loadHistory())
  type SourceMode = 'auto' | 'whois_verisign' | 'whois_cndns' | 'whois_hichina'
  const [sourceMode, setSourceMode] = useState<SourceMode>('auto')

  const domains = useMemo(() => {
    const list = input
      .split(/[\s,;\n]+/)
      .map((s) => s.trim().toLowerCase())
      .filter(
        (s) =>
          s &&
          /^(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+[a-z]{2,}$/i.test(s),
      )
    return Array.from(new Set(list))
  }, [input])

  const stats: ProgressStats = useMemo(() => {
    const arr = Object.values(results)
    const total = arr.length
    const completed = arr.filter((r) => r.finishedAt).length
    const success = arr.filter((r) => r.best).length
    const avgMs = (() => {
      const times = arr
        .filter((r) => r.finishedAt && r.startedAt)
        .map((r) => r.finishedAt! - r.startedAt!)
      if (times.length === 0) return 0
      return Math.round(times.reduce((a, b) => a + b, 0) / times.length)
    })()
    return { total, completed, success, avgMs }
  }, [results])

  const runSingle = useCallback(
    async (domain: string) => {
      const startedAt = Date.now()
      setResults((prev) => ({
        ...prev,
        [domain]: { domain, channels: [], startedAt },
      }))

      // Cache hit (keyed by domain+source)
      const cacheKey = `${domain}|${sourceMode}`
      const cached = cacheRef.current[cacheKey]
      if (cached && Date.now() - cached.ts < CACHE_TTL_MS) {
        setResults((prev) => ({
          ...prev,
          [domain]: { ...cached.data, startedAt, finishedAt: Date.now() },
        }))
        return
      }

      const channels: WhoisParsed[] = []
      let errorAggregate: string[] = []
      try {
        if (sourceMode === 'auto') {
          const parsed: WhoisParsed = await invoke('query_whois_unified', {
            domain,
            source: 'auto',
          })
          channels.push(parsed)
        } else if (sourceMode === 'whois_verisign') {
          await limiter.wait('whois_verisign')
          const parsed: WhoisParsed = await invoke('query_whois_unified', {
            domain,
            source: 'whois.verisign-grs.com',
          })
          channels.push(parsed)
        } else if (sourceMode === 'whois_cndns') {
          await limiter.wait('whois_cndns')
          const parsed: WhoisParsed = await invoke('query_whois_unified', {
            domain,
            source: 'grs-whois.cndns.com',
          })
          channels.push(parsed)
        } else if (sourceMode === 'whois_hichina') {
          await limiter.wait('whois_hichina')
          const parsed: WhoisParsed = await invoke('query_whois_unified', {
            domain,
            source: 'grs-whois.hichina.com',
          })
          channels.push(parsed)
        }
      } catch (e: any) {
        errorAggregate.push(`查询失败: ${e?.toString?.() ?? e}`)
      }

      const best = pickBest(channels)
      const finishedAt = Date.now()
      const data: DomainResult = {
        domain,
        channels,
        best,
        error: errorAggregate.length ? errorAggregate.join('；') : undefined,
        startedAt,
        finishedAt,
      }

      // 仅成功才写缓存和历史
      if (best) {
        cacheRef.current[cacheKey] = { ts: Date.now(), data }
        saveCache(cacheRef.current)
        setHistory((prev) => {
          const next = [...prev, data]
          saveHistory(next)
          return next
        })
      }
      setResults((prev) => ({ ...prev, [domain]: data }))
    },
    [limiter, sourceMode],
  )

  const runBatch = useCallback(async () => {
    if (domains.length === 0) return
    setRunning(true)
    setResults({})
    const concurrency = 6
    let idx = 0

    const workers = new Array(concurrency).fill(0).map(async () => {
      while (idx < domains.length) {
        const d = domains[idx++]
        await runSingle(d)
      }
    })
    await Promise.all(workers)
    setRunning(false)
  }, [domains, runSingle])

  const toggleSelect = (domain: string) => {
    setSelected((prev) => {
      const s = new Set(prev)
      if (s.has(domain)) s.delete(domain)
      else s.add(domain)
      return s
    })
  }

  const clearHistory = () => {
    setHistory([])
    saveHistory([])
  }

  const selectedResults = useMemo(() => {
    return Array.from(selected)
      .map((d) => results[d])
      .filter(Boolean) as DomainResult[]
  }, [selected, results])

  return (
    <ToolLayout
      title='域名 Whois 查询（多源）'
      description='查询域名的 Whois 信息，支持多个数据源'
      padding={false}>
      <div className='space-y-4 text-gray-900 dark:text-gray-100 p-4'>
        <div className='grid grid-cols-1 md:grid-cols-2 gap-4'>
          <div>
            <label className='block text-sm text-gray-600 dark:text-gray-300 mb-1'>
              批量域名（每行一个或用逗号/空格分隔）
            </label>
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder={'例如:\nexample.com\nopenai.com\ncloudflare.net'}
              className='w-full h-40 p-2 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-sm text-gray-900 dark:text-gray-100'
            />
            <div className='mt-2 flex items-center gap-2'>
              <span className='text-sm text-gray-700 dark:text-gray-200'>
                查询源：
              </span>
              <select
                value={sourceMode}
                onChange={(e) => setSourceMode(e.target.value as any)}
                className='px-2 py-1 rounded border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-sm text-gray-900 dark:text-gray-100'>
                <option value='auto'>自动（verisign → cndns → hichina）</option>
                <option value='whois_verisign'>whois.verisign-grs.com</option>
                <option value='whois_cndns'>grs-whois.cndns.com</option>
                <option value='whois_hichina'>grs-whois.hichina.com</option>
              </select>
            </div>
            <div className='mt-2 flex gap-2'>
              <button
                onClick={runBatch}
                disabled={running || domains.length === 0}
                className='px-3 py-2 rounded bg-blue-600 text-white text-sm disabled:opacity-50'>
                {running ? '查询中...' : `开始查询 (${domains.length})`}
              </button>
              <button
                onClick={() => exportCSV(Object.values(results))}
                disabled={Object.keys(results).length === 0}
                className='px-3 py-2 rounded bg-green-600 text-white text-sm disabled:opacity-50'>
                导出 CSV
              </button>
              <button
                onClick={() => exportJSON(Object.values(results))}
                disabled={Object.keys(results).length === 0}
                className='px-3 py-2 rounded bg-green-700 text-white text-sm disabled:opacity-50'>
                导出 JSON
              </button>
              <button
                onClick={clearHistory}
                className='px-3 py-2 rounded bg-gray-300 dark:bg-gray-600 text-sm'>
                清空历史
              </button>
            </div>
            <div className='mt-2 text-xs text-gray-600 dark:text-gray-400'>
              频率限制：WHOIS 每 700ms（按服务器区分）。缓存有效期 24
              小时。失败不记录历史。
            </div>
          </div>
          <div className='p-3 rounded border border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800'>
            <div className='text-sm text-gray-800 dark:text-gray-200'>
              <div>总数：{stats.total}</div>
              <div>已完成：{stats.completed}</div>
              <div>
                成功：{stats.success}（成功率{' '}
                {stats.total
                  ? Math.round((stats.success / stats.total) * 100)
                  : 0}
                %）
              </div>
              <div>平均响应时间：{stats.avgMs} ms</div>
            </div>
          </div>
        </div>

        <div className='overflow-x-auto'>
          <table className='min-w-full text-sm text-gray-800 dark:text-gray-200'>
            <thead>
              <tr className='text-left border-b border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300'>
                <th className='p-2'>选择</th>
                <th className='p-2'>域名</th>
                <th className='p-2'>来源</th>
                <th className='p-2'>注册商</th>
                <th className='p-2'>注册人</th>
                <th className='p-2'>注册日期</th>
                <th className='p-2'>过期日期</th>
                <th className='p-2'>状态</th>
                <th className='p-2'>NS 数</th>
                <th className='p-2'>NS 列表</th>
                <th className='p-2'>错误</th>
              </tr>
            </thead>
            <tbody>
              {Object.values(results).map((r) => (
                <tr
                  key={r.domain}
                  className='border-b border-gray-100 dark:border-gray-700'>
                  <td className='p-2'>
                    <input
                      type='checkbox'
                      checked={selected.has(r.domain)}
                      onChange={() => toggleSelect(r.domain)}
                    />
                  </td>
                  <td className='p-2 font-medium text-gray-900 dark:text-gray-100'>
                    {r.domain}
                  </td>
                  <td className='p-2'>{r.best?.source ?? '—'}</td>
                  <td className='p-2'>{r.best?.registrar ?? '—'}</td>
                  <td className='p-2'>{r.best?.registrant ?? '—'}</td>
                  <td className='p-2'>{r.best?.created ?? '—'}</td>
                  <td className='p-2'>{r.best?.expires ?? '—'}</td>
                  <td className='p-2'>
                    {(r.best?.status && r.best?.status.join('|')) || '—'}
                  </td>
                  <td className='p-2'>{r.best?.nameServers?.length ?? 0}</td>
                  <td className='p-2'>
                    {r.best?.nameServers && r.best.nameServers.length > 0 ? (
                      <div className='max-w-[240px] truncate'>
                        {r.best.nameServers.slice(0, 3).join(', ')}
                        {r.best.nameServers.length > 3 ? ' …' : ''}
                      </div>
                    ) : (
                      '—'
                    )}
                  </td>
                  <td className='p-2 text-red-600 dark:text-red-400'>
                    {r.error ?? ''}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {selectedResults.length >= 2 && (
          <div className='mt-4'>
            <h3 className='text-lg font-semibold'>
              结果对比（{selectedResults.length}）
            </h3>
            <div className='overflow-x-auto'>
              <table className='min-w-full text-sm text-gray-800 dark:text-gray-200'>
                <thead>
                  <tr className='text-left border-b border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-300'>
                    <th className='p-2'>字段</th>
                    {selectedResults.map((r) => (
                      <th key={r.domain} className='p-2'>
                        {r.domain}
                      </th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {[
                    'source',
                    'registrar',
                    'registrant',
                    'created',
                    'expires',
                    'updated',
                    'status',
                    'nameServers',
                  ].map((field) => (
                    <tr
                      key={field}
                      className='border-b border-gray-100 dark:border-gray-700'>
                      <td className='p-2 font-medium text-gray-900 dark:text-gray-100'>
                        {field}
                      </td>
                      {selectedResults.map((r) => {
                        const b = r.best
                        let val: any = (b as any)?.[field]
                        if (Array.isArray(val)) val = val.join('|')
                        return (
                          <td key={r.domain + field} className='p-2'>
                            {val ?? '—'}
                          </td>
                        )
                      })}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        <div className='mt-6'>
          <h3 className='text-lg font-semibold'>查询历史</h3>
          <div className='grid grid-cols-1 md:grid-cols-2 gap-3'>
            {history
              .slice()
              .reverse()
              .slice(0, 20)
              .map((h, idx) => (
                <div
                  key={idx}
                  className='p-2 rounded border border-gray-200 dark:border-gray-700'>
                  <div className='text-sm font-medium text-gray-900 dark:text-gray-100'>
                    {h.domain}
                  </div>
                  <div className='text-xs text-gray-600 dark:text-gray-400'>
                    来源：{h.best?.source ?? '—'}；注册商：
                    {h.best?.registrar ?? '—'}
                  </div>
                  <div className='text-xs text-gray-600 dark:text-gray-400'>
                    注册：{h.best?.created ?? '—'}；过期：
                    {h.best?.expires ?? '—'}
                  </div>
                </div>
              ))}
          </div>
        </div>
      </div>
    </ToolLayout>
  )
}

export default WhoisLookup
