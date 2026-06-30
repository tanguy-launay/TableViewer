import { ref, computed, watch } from 'vue'

export const ZOOM_STEP = 10
export const ZOOM_MIN  = 60
export const ZOOM_MAX  = 200

export function useThemeZoom() {
    const isDark = ref(false)
    const zoom   = ref(100)

    watch(isDark, dark => {
        document.body.style.background = dark ? '#18181c' : '#ffffff'
    }, { immediate: true })

    const appStyle = computed(() => ({ fontSize: `${zoom.value}%` }))

    function zoomIn()    { zoom.value = Math.min(zoom.value + ZOOM_STEP, ZOOM_MAX) }
    function zoomOut()   { zoom.value = Math.max(zoom.value - ZOOM_STEP, ZOOM_MIN) }
    function zoomReset() { zoom.value = 100 }

    function handleZoomKey(e: KeyboardEvent) {
        if (!e.ctrlKey) return
        if (e.key === '+' || e.key === '=') { e.preventDefault(); zoomIn() }
        if (e.key === '-')                   { e.preventDefault(); zoomOut() }
        if (e.key === '0')                   { e.preventDefault(); zoomReset() }
    }

    return { isDark, zoom, appStyle, zoomIn, zoomOut, zoomReset, handleZoomKey }
}
