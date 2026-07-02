<template>
    <n-modal
        v-model:show="show"
        preset="card"
        title="Preprocess with jaq"
        style="width: min(860px, 92vw); max-height: 88vh;"
        :segmented="{ content: true }"
    >
        <div class="jaq-header">
            <n-tag :type="tagColor" size="small">{{ filetype }}</n-tag>
            <span class="jaq-paths" :title="paths.join(', ')">
                {{ paths.map(p => p.split('/').pop()).join(', ') }}
            </span>
        </div>

        <div class="jaq-row">
            <n-input
                v-model:value="filter"
                placeholder=". or .results[] or map(del(.internal))"
                type="textarea"
                :rows="3"
                style="font-family: monospace; flex: 1;"
                @keyup.ctrl.enter="runPreview"
            />
            <n-button :loading="loading" @click="runPreview">Preview</n-button>
        </div>

        <div v-if="errMsg" class="jaq-error">{{ errMsg }}</div>

        <pre v-if="preview" class="jaq-preview">{{ preview }}</pre>
        <div v-if="totalRows !== null" class="jaq-info">
            {{ totalRows }} row{{ totalRows !== 1 ? 's' : '' }} total · showing first {{ previewCount }}
        </div>

        <template #footer>
            <div style="display: flex; justify-content: flex-end; gap: 8px;">
                <n-button @click="show = false">Cancel</n-button>
                <n-button
                    type="primary"
                    :disabled="!!errMsg || totalRows === 0"
                    @click="confirm"
                >
                    Import
                </n-button>
            </div>
        </template>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NButton, NInput, NTag } from 'naive-ui'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
    modelValue: boolean
    paths: string[]
    filetype: string
}>()

const emit = defineEmits<{
    (e: 'update:modelValue', v: boolean): void
    (e: 'confirm', filter: string): void
}>()

const show = computed({
    get: () => props.modelValue,
    set: v  => emit('update:modelValue', v),
})

const filter      = ref('.')
const preview     = ref('')
const errMsg      = ref('')
const loading     = ref(false)
const totalRows   = ref<number | null>(null)
const previewCount = ref(0)

const tagColor = computed((): 'success' | 'info' | 'warning' | 'error' | 'default' => {
    const ft = props.filetype
    if (ft === 'json' || ft === 'jsonl') return 'error'
    if (ft === 'yaml' || ft === 'yml')   return 'info'
    if (ft === 'xml')                    return 'warning'
    return 'default'
})

watch(
    () => props.modelValue,
    open => {
        if (open) {
            filter.value  = '.'
            preview.value = ''
            errMsg.value  = ''
            totalRows.value = null
            runPreview()
        }
    },
)

async function runPreview() {
    if (!props.paths.length) return
    loading.value = true
    errMsg.value  = ''
    preview.value = ''

    try {
        const res: any = JSON.parse(
            await invoke<string>('preview_jaq', {
                paths:    props.paths,
                filetype: props.filetype,
                filter:   filter.value,
            })
        )
        if ('err_msg' in res) {
            errMsg.value    = res.err_msg
            totalRows.value = null
        } else {
            totalRows.value  = res.total
            previewCount.value = res.preview.length
            preview.value    = JSON.stringify(res.preview, null, 2)
            errMsg.value     = ''
        }
    } catch (e: any) {
        errMsg.value = String(e)
    } finally {
        loading.value = false
    }
}

function confirm() {
    emit('confirm', filter.value)
    show.value = false
}
</script>

<style scoped>
.jaq-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
    font-size: 12px;
    opacity: 0.7;
}

.jaq-paths {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
}

.jaq-row {
    display: flex;
    gap: 8px;
    align-items: flex-start;
    margin-bottom: 10px;
}

.jaq-error {
    color: #e88080;
    font-size: 12px;
    margin-bottom: 8px;
    font-family: monospace;
    white-space: pre-wrap;
}

.jaq-preview {
    background: rgba(0, 0, 0, 0.12);
    border-radius: 4px;
    padding: 10px 12px;
    font-size: 12px;
    max-height: 280px;
    overflow-y: auto;
    margin-bottom: 6px;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.5;
}

.jaq-info {
    font-size: 11px;
    opacity: 0.5;
    text-align: right;
    margin-bottom: 4px;
}
</style>
