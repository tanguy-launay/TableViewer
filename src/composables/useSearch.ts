import { ref, computed, watch, h } from 'vue'
import type { Ref } from 'vue'
import type { SearchMode } from '../types'

export const SEARCH_MODES = [
    { value: 'text'  as SearchMode, label: 'T',  title: 'Full-text search'    },
    { value: 'fuzzy' as SearchMode, label: '≈',  title: 'Fuzzy search'        },
    { value: 'regex' as SearchMode, label: '.*', title: 'Regular expression'  },
]

export function useSearch(tbHeaders: Ref<any[]>, tbBody: Ref<any[]>) {
    const searchText  = ref('')
    const searchMode  = ref<SearchMode>('text')
    const currentPage = ref(1)

    watch(searchText, () => { currentPage.value = 1 })

    function matches(value: string, term: string): boolean {
        if (!term) return true
        const v = value.toLowerCase()
        const t = term.toLowerCase()
        if (searchMode.value === 'text')  return v.includes(t)
        if (searchMode.value === 'fuzzy') {
            let ti = 0
            for (let i = 0; i < v.length && ti < t.length; i++)
                if (v[i] === t[ti]) ti++
            return ti === t.length
        }
        if (searchMode.value === 'regex') {
            try { return new RegExp(term, 'i').test(value) } catch { return false }
        }
        return false
    }

    function highlight(value: string, term: string) {
        if (!term || !matches(value, term)) return h('span', value)

        if (searchMode.value === 'text') {
            const idx = value.toLowerCase().indexOf(term.toLowerCase())
            if (idx === -1) return h('span', value)
            return h('span', [
                value.slice(0, idx),
                h('mark', { class: 'hl' }, value.slice(idx, idx + term.length)),
                value.slice(idx + term.length),
            ])
        }

        if (searchMode.value === 'regex') {
            try {
                const re = new RegExp(term, 'gi')
                const parts: any[] = []
                let last = 0, m: RegExpExecArray | null
                while ((m = re.exec(value)) !== null) {
                    if (m.index > last) parts.push(value.slice(last, m.index))
                    parts.push(h('mark', { class: 'hl' }, m[0]))
                    last = re.lastIndex
                    if (m[0].length === 0) { re.lastIndex++; break }
                }
                if (last < value.length) parts.push(value.slice(last))
                return h('span', parts)
            } catch { return h('span', value) }
        }

        // fuzzy: highlight each matched character
        const t = term.toLowerCase()
        const parts: any[] = []
        let ti = 0, buf = ''
        for (let i = 0; i < value.length; i++) {
            if (ti < t.length && value[i].toLowerCase() === t[ti]) {
                if (buf) { parts.push(buf); buf = '' }
                parts.push(h('mark', { class: 'hl' }, value[i]))
                ti++
            } else { buf += value[i] }
        }
        if (buf) parts.push(buf)
        return h('span', parts)
    }

    const filteredBody = computed(() => {
        const term = searchText.value.trim()
        if (!term) return tbBody.value
        return tbBody.value.filter((row: Record<string, string>) =>
            Object.values(row).some(v => matches(String(v), term))
        )
    })

    const renderedHeaders = computed(() => {
        const term = searchText.value.trim()
        return tbHeaders.value.map((col: any) => {
            if (!term) return col
            return {
                ...col,
                render: (row: Record<string, string>) =>
                    highlight(String(row[col.key] ?? ''), term),
            }
        })
    })

    return { searchText, searchMode, currentPage, filteredBody, renderedHeaders }
}
