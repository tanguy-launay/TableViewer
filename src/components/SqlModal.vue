<template>
    <n-modal
        v-model:show="showModal"
        preset="card"
        title="SQL Editor — Ctrl+Enter to execute"
        style="width: min(940px, 92vw)"
        :segmented="{ content: true, footer: 'soft' }"
    >
        <vue-monaco-editor
            v-model:value="localSql"
            language="sql"
            :theme="isDark ? 'vs-dark' : 'vs'"
            height="420px"
            :options="EDITOR_OPTIONS"
            @keydown="onKeydown"
        />
        <template #footer>
            <div class="modal-footer">
                <n-button @click="showModal = false">Cancel</n-button>
                <n-button type="primary" @click="run">▶ Execute</n-button>
            </div>
        </template>
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { NModal, NButton } from 'naive-ui'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'

const props = defineProps<{
    modelValue: boolean   // v-model: show/hide
    sql: string
    isDark: boolean
}>()

const emit = defineEmits<{
    (e: 'update:modelValue', v: boolean): void
    (e: 'execute', sql: string): void
}>()

const showModal = computed({
    get: () => props.modelValue,
    set: v  => emit('update:modelValue', v),
})

const localSql = ref(props.sql)

// Sync editor content whenever the modal opens
watch(() => props.modelValue, open => {
    if (open) localSql.value = props.sql
})

const EDITOR_OPTIONS = {
    minimap:              { enabled: false },
    fontSize:             14,
    lineNumbers:          'on' as const,
    automaticLayout:      true,
    wordWrap:             'on' as const,
    scrollBeyondLastLine: false,
    tabSize:              2,
}

function run() {
    emit('execute', localSql.value)
    showModal.value = false
}

function onKeydown(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === 'Enter') {
        e.preventDefault()
        run()
    }
}
</script>

<style scoped>
.modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 4px 0;
}
</style>
