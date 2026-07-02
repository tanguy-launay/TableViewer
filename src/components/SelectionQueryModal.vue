<template>
    <n-modal
        v-model:show="localShow"
        preset="card"
        title="Query"
        style="width: min(860px, 92vw); max-height: 92vh;"
        :segmented="{ content: true }"
    >
        <vue-monaco-editor
            v-model:value="sqlText"
            language="sql"
            theme="vs-dark"
            height="300px"
            :options="EDITOR_OPTIONS"
        />
    </n-modal>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { NModal } from 'naive-ui'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'

const props = defineProps<{ show: boolean }>()
const emit  = defineEmits<{ (e: 'update:show', v: boolean): void }>()

const localShow = computed({
    get: () => props.show,
    set: v => emit('update:show', v),
})

const sqlText = ref('')

const EDITOR_OPTIONS = {
    minimap:              { enabled: false },
    fontSize:             13,
    wordWrap:             'on' as const,
    scrollBeyondLastLine: false,
    automaticLayout:      true,
}
</script>
