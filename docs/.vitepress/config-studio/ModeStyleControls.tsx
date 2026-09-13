import { cardPositionRatios } from '../simulator/window-card-position.ts'
import CardPositionEditor from './CardPositionEditor'
import CardStylePreview from './CardStylePreview'
import { parseSplitRatios } from '../simulator/window-ratios.ts'
import { computed, defineComponent } from 'vue'
import {
  cloneConfigDocument,
  deleteConfigPath,
  getConfigPath,
  setConfigPath,
  type ConfigDocument,
} from './document'

type TargetingMode = 'grid' | 'recursive_grid' | 'ui_hint' | 'key_help' | 'window' | 'window_quick' | 'window_editor' | 'window_restore' | 'window_tab'
type Appearance = 'dark' | 'light'
type ControlKind = 'color' | 'number' | 'text' | 'boolean' | 'select' | 'ratios' | 'percentages'

interface StyleField {
  path: string
  label: string
  kind: ControlKind
  min?: number
  max?: number
  step?: number
  options?: string[]
}

interface ModeFields {
  colors: StyleField[]
  layout: StyleField[]
  advanced: StyleField[]
}

const fields = {
  window: {
    colors: [
      { path: 'key_help.background_color', label: '面板背景（共用 key_help）', kind: 'color' },
      { path: 'key_help.text_color', label: '面板文字（共用 key_help）', kind: 'color' },
      { path: 'window.ui.border_color', label: '目标描边', kind: 'color' },
    ],
    layout: [
      { path: 'window.screens', label: '窗口范围（current 当前屏幕 / all 全部屏幕）', kind: 'select', options: ['current', 'all'] },
      { path: 'window.include_minimized', label: '包含最小化窗口', kind: 'boolean' },
      { path: 'window.enabled', label: '启用 Window', kind: 'boolean' },
      { path: 'window.number_timeout_ms', label: '歧义编号等待（毫秒）', kind: 'number', min: 100, max: 2000, step: 10 },
      { path: 'window.move_step', label: '移动步长', kind: 'number', min: 0, max: 10000, step: 1 },
      { path: 'window.resize_step', label: '缩放步长', kind: 'number', min: 0, max: 10000, step: 1 },
      { path: 'window.border_width', label: '目标描边宽度', kind: 'number', min: 0, max: 20, step: .5 },
    ],
    advanced: [
      { path: 'window.move_speed', label: '移动速度', kind: 'number', min: 0, max: 10000, step: 10 },
      { path: 'window.resize_speed', label: '缩放速度', kind: 'number', min: 0, max: 10000, step: 10 },
      { path: 'key_help.font_family', label: '面板字体（共用 key_help）', kind: 'text' },
      { path: 'key_help.font_size', label: '面板字号（共用 key_help）', kind: 'number', min: 1, max: 72, step: 1 },
    ],
  },
  key_help: {
    colors: [
      { path: 'key_help.background_color', label: '背景色', kind: 'color' },
      { path: 'key_help.text_color', label: '文字色', kind: 'color' },
      { path: 'key_help.border_color', label: '边框色', kind: 'color' },
    ],
    layout: [
  { path: 'key_help.mouse_key_help', label: '鼠标模式默认显示提示（? 可切换）', kind: 'boolean' },
  { path: 'key_help.window_key_help', label: '窗口模式默认显示提示（? 可切换）', kind: 'boolean' },
      { path: 'key_help.font_size', label: '字号', kind: 'number', min: 1, max: 72, step: 1 },
      { path: 'key_help.padding_x', label: '水平内边距', kind: 'number', min: 0, max: 100, step: 1 },
      { path: 'key_help.padding_y', label: '垂直内边距', kind: 'number', min: 0, max: 100, step: 1 },
    ],
    advanced: [
      { path: 'key_help.font_family', label: '字体（空值继承）', kind: 'text' },
      { path: 'key_help.border_width', label: '边框宽度', kind: 'number', min: 0, max: 20, step: 1 },
      { path: 'key_help.border_radius', label: '圆角', kind: 'number', min: 0, max: 100, step: 1 },
    ],
  },
  grid: {
    colors: [
      { path: 'grid.ui.background_color', label: '标签底色', kind: 'color' },
      { path: 'grid.ui.text_color', label: '文字色', kind: 'color' },
      { path: 'grid.ui.border_color', label: '标签边框', kind: 'color' },
      { path: 'grid.ui.matched_background_color', label: '选中填充', kind: 'color' },
      { path: 'grid.ui.matched_border_color', label: '网格线', kind: 'color' },
    ],
    layout: [
      { path: 'grid.grid_cols', label: '列数', kind: 'number', min: 1, max: 12, step: 1 },
      { path: 'grid.grid_rows', label: '行数', kind: 'number', min: 1, max: 12, step: 1 },
      { path: 'grid.keys', label: '网格键', kind: 'text' },
      { path: 'grid.ui.font_size', label: '字号', kind: 'number', min: 6, max: 72, step: 1 },
    ],
    advanced: [
      { path: 'grid.max_depth', label: '最大层数', kind: 'number', min: 1, max: 20, step: 1 },
      { path: 'grid.ui.font_family', label: '字体', kind: 'text' },
      { path: 'grid.ui.border_width', label: '线宽', kind: 'number', min: 0.5, max: 8, step: 0.5 },
    ],
  },
  recursive_grid: {
    colors: [
      { path: 'recursive_grid.ui.line_color', label: '网格线', kind: 'color' },
      { path: 'recursive_grid.ui.highlight_color', label: '高亮色', kind: 'color' },
      { path: 'recursive_grid.ui.label_background_color', label: '标签填充', kind: 'color' },
      { path: 'recursive_grid.ui.text_color', label: '文字色', kind: 'color' },
      { path: 'recursive_grid.ui.sub_key_preview_text_color', label: '小字母颜色', kind: 'color' },
    ],
    layout: [
      { path: 'recursive_grid.grid_cols', label: '列数', kind: 'number', min: 1, max: 8, step: 1 },
      { path: 'recursive_grid.grid_rows', label: '行数', kind: 'number', min: 1, max: 8, step: 1 },
      { path: 'recursive_grid.keys', label: '网格键', kind: 'text' },
      { path: 'recursive_grid.ui.font_size', label: '大字母字号', kind: 'number', min: 6, max: 72, step: 1 },
      { path: 'recursive_grid.ui.sub_key_preview_font_size', label: '小字母字号', kind: 'number', min: 4, max: 24, step: 1 },
    ],
    advanced: [
      { path: 'recursive_grid.ui.font_family', label: '字体', kind: 'text' },
      { path: 'recursive_grid.ui.label_min_font_size', label: '最小字号', kind: 'number', min: 1, max: 32, step: 1 },
      { path: 'recursive_grid.ui.line_width', label: '线宽', kind: 'number', min: 0.5, max: 8, step: 0.5 },
      { path: 'recursive_grid.ui.label_background', label: '标签底色', kind: 'boolean' },
      { path: 'recursive_grid.ui.label_char', label: '替代字符', kind: 'text' },
      { path: 'recursive_grid.ui.sub_key_preview', label: '下层预览', kind: 'boolean' },
    ],
  },
  ui_hint: {
    colors: [
      { path: 'ui_hint.ui.background_color', label: '标签底色', kind: 'color' },
      { path: 'ui_hint.ui.text_color', label: '文字色', kind: 'color' },
      { path: 'ui_hint.ui.matched_text_color', label: '匹配文字', kind: 'color' },
      { path: 'ui_hint.ui.border_color', label: '边框色', kind: 'color' },
      { path: 'ui_hint.boundary_highlight.background_color', label: '轮廓填充', kind: 'color' },
      { path: 'ui_hint.boundary_highlight.border_color', label: '轮廓颜色', kind: 'color' },
    ],
    layout: [
      { path: 'ui_hint.hint_characters', label: '提示键', kind: 'text' },
      { path: 'ui_hint.placement', label: '标签位置', kind: 'select', options: ['top', 'center', 'bottom'] },
      { path: 'ui_hint.ui.font_size', label: '字号', kind: 'number', min: 6, max: 72, step: 1 },
      { path: 'ui_hint.ui.border_width', label: '边框', kind: 'number', min: 0, max: 8, step: 0.5 },
    ],
    advanced: [
      { path: 'ui_hint.label_x_offset', label: '水平偏移', kind: 'number', min: -100, max: 100, step: 1 },
      { path: 'ui_hint.label_y_offset', label: '垂直偏移', kind: 'number', min: -100, max: 100, step: 1 },
      { path: 'ui_hint.ui.font_family', label: '字体', kind: 'text' },
      { path: 'ui_hint.ui.border_radius', label: '圆角', kind: 'number', min: -1, max: 32, step: 1 },
      { path: 'ui_hint.ui.padding_x', label: '水平内边距', kind: 'number', min: -1, max: 32, step: 1 },
      { path: 'ui_hint.ui.padding_y', label: '垂直内边距', kind: 'number', min: -1, max: 32, step: 1 },
      { path: 'ui_hint.boundary_highlight.enabled', label: '元素轮廓', kind: 'boolean' },
      { path: 'ui_hint.boundary_highlight.border_width', label: '轮廓线宽', kind: 'number', min: 0, max: 8, step: 0.5 },
    ],
  },
} as Record<TargetingMode, ModeFields>

fields.window.layout.push({ path: 'window.card.position_mode', label: '卡片定位（window 窗口 / screen 当前屏幕）', kind: 'select', options: ['window', 'screen'] })
fields.window.layout.push({ path: 'window.card.position', label: '上、右、下、左（四个百分比，逗号分隔）', kind: 'percentages' })
fields.window.colors.push({ path: 'window.card.border_color', label: '卡片边框颜色', kind: 'color' })
fields.window.colors.push({ path: 'window.card.background_color', label: '卡片背景', kind: 'color' })
fields.window.colors.push({ path: 'window.card.number_color', label: '编号文字', kind: 'color' })
fields.window.colors.push({ path: 'window.card.app_color', label: '程序名颜色', kind: 'color' })
fields.window.colors.push({ path: 'window.card.title_color', label: '标题颜色', kind: 'color' })
fields.window.advanced.push({ path: 'window.ui.font_size', label: '编号字号', kind: 'number', min: 1, max: 256 })
fields.window.advanced.push({ path: 'window.ui.font_family', label: '编号字体', kind: 'text' })
fields.window.advanced.push({ path: 'window.ui.border_radius', label: '卡片圆角（-1 自动）', kind: 'number', min: -1, max: 256 })
fields.window.advanced.push({ path: 'window.ui.border_width', label: '卡片边框宽度', kind: 'number', min: 0, max: 20 })
fields.window.advanced.push({ path: 'window.card.app_font_size', label: '程序名字号（0 自动）', kind: 'number', min: 0, max: 256 })
fields.window.advanced.push({ path: 'window.card.title_font_size', label: '标题字号（0 自动）', kind: 'number', min: 0, max: 256 })
fields.window.advanced.push({ path: 'window.card.app_font_family', label: '程序名字体（空值继承）', kind: 'text' })
fields.window.advanced.push({ path: 'window.card.title_font_family', label: '标题字体（空值继承）', kind: 'text' })
fields.window.advanced.push({ path: 'window.card.app_bold', label: '程序名加粗', kind: 'boolean' })
fields.window.advanced.push({ path: 'window.card.title_bold', label: '标题加粗', kind: 'boolean' })
fields.window.advanced.push({ path: 'window.card.text_width', label: '每列文字宽度', kind: 'number', min: 1, max: 4096 })
fields.window.advanced.push({ path: 'window.card.padding_x', label: '文字水平内边距', kind: 'number', min: 0, max: 256 })
fields.window.advanced.push({ path: 'window.card.padding_y', label: '文字垂直内边距', kind: 'number', min: 0, max: 256 })
fields.window.advanced.push({ path: 'window.card.line_height', label: '文字行高倍数', kind: 'number', min: 1, max: 4, step: 0.1 })
fields.window.advanced.push({ path: 'window.card.min_height', label: '卡片最小高度', kind: 'number', min: 0, max: 4096 })
fields.window.advanced.push({ path: 'window.card.number_min_width', label: '编号最小宽度', kind: 'number', min: 0, max: 4096 })
fields.window.advanced.push({ path: 'window.ui.padding_x', label: '编号水平内边距（-1 自动）', kind: 'number', min: -1, max: 256 })
fields.window.advanced.push({ path: 'window.ui.padding_y', label: '编号垂直内边距（-1 自动）', kind: 'number', min: -1, max: 256 })

for (const mode of ['window_quick', 'window_editor', 'window_restore', 'window_tab'] as const) {
  const common = (items: StyleField[]) => items.filter(f => !/window\.(move_|resize_)/.test(f.path) && (mode === 'window_editor' || !f.path.startsWith('window.card.position'))).map(f => ({ ...f, path: f.path.replace(/^window\.(?!card\.(?!position))/, `${mode}.`) }))
  ;(fields as Record<string, ModeFields>)[mode] = { colors: common(fields.window.colors), layout: common(fields.window.layout), advanced: common(fields.window.advanced) }
  const own = (fields as Record<string, ModeFields>)[mode]
  if (mode === 'window_quick') own.layout.push({ path: `${mode}.split_ratios`, label: '比例（逗号分隔，支持分数）', kind: 'ratios' })
  if (mode !== 'window_tab') own.layout.push({ path: `${mode}.gap`, label: '布局间距', kind: 'number', min: 0, step: 1 })
  if (mode === 'window_editor') own.advanced.push({ path: `${mode}.resize_step`, label: '分割线步长', kind: 'number', min: 0 }, { path: `${mode}.resize_speed`, label: '分割线速度', kind: 'number', min: 0 })
  if (mode === 'window_restore') own.advanced.push({ path: `${mode}.lifecycle.after_finish`, label: '恢复成功后', kind: 'select', options: ['window_editor', 'window', 'window_restore', 'keep', 'normal', 'idle'] })
}

const paletteFields = ['surface', 'accent', 'accent_alt', 'on_accent_alt', 'text'] as const
const paletteLabels: Record<(typeof paletteFields)[number], string> = {
  surface: '表面',
  accent: '主色',
  accent_alt: '高亮',
  on_accent_alt: '高亮文字',
  text: '文字',
}

export default defineComponent({
  name: 'ModeStyleControls',
  props: {
    document: { type: Object as () => ConfigDocument, required: true },
    effectiveDocument: { type: Object as () => ConfigDocument, required: true },
    mode: { type: String as () => TargetingMode, required: true },
    appearance: { type: String as () => Appearance, required: true },
  },
  emits: {
    change: (_document: ConfigDocument) => true,
    appearanceChange: (_appearance: Appearance) => true,
  },
  setup(props, { emit }) {
    const positionRoot = computed(() => props.mode === 'window_editor' ? 'window_editor.card' : 'window.card')
    const modeFields = computed(() => fields[props.mode])
    const isCardField = (field: StyleField) => props.mode.startsWith('window') &&
      ((field.path.startsWith('window.card.') || field.path.startsWith('window_editor.card.')) || /^window(?:_\w+)?\.ui\./.test(field.path))
    const fieldValue = (field: StyleField): unknown => {
      const configured = getConfigPath(props.effectiveDocument, field.path)
      if (configured !== undefined || !isCardField(field)) return configured
      if (field.path.endsWith('.ui.border_width')) return 1
      if (field.kind !== 'color') return undefined
      const ui = props.effectiveDocument[props.mode]?.ui ?? {}
      const key = field.path.split('.').at(-1)
      const source = key === 'background_color' ? 'surface' : key === 'border_color' ? 'accent' : 'text'
      const inherited = ui[key === 'background_color' ? 'background_color' : key === 'border_color' ? 'border_color' : 'text_color']
      return Object.fromEntries(['light', 'dark'].map(appearance => [appearance,
        (typeof inherited === 'string' ? inherited : inherited?.[appearance]) ?? props.effectiveDocument.theme?.[appearance]?.[source] ?? (appearance === 'dark' ? '#E8EEFFFF' : '#17327AFF')]))
    }

    function update(path: string, value: unknown): void {
      const next = cloneConfigDocument(props.document)
      setConfigPath(next, path, value)
      emit('change', next)
    }

    function reset(path: string): void {
      const next = cloneConfigDocument(props.document)
      deleteConfigPath(next, path)
      emit('change', next)
    }

    const renderFields = (items: StyleField[]) => (
      <div class="ks-style-fields">
        {items.map((field) => (
          <StyleControl
            field={field}
            value={fieldValue(field)}
            appearance={props.appearance}
            inherited={getConfigPath(props.document, field.path) === undefined}
            onUpdate={(value) => update(field.path, value)}
            onReset={() => reset(field.path)}
          />
        ))}
      </div>
    )

    return () => {
      const palette = paletteFields.map<StyleField>((field) => ({
        path: `theme.${props.appearance}.${field}`,
        label: paletteLabels[field],
        kind: 'color',
      }))
      return (
        <div class="ks-style-controls">
          <div class="ks-style-heading">
            <div><strong>颜色与 {modeLabel(props.mode)} 样式</strong><span>配置值会立即写入 TOML 并显示在上方预览中</span></div>
            <div class="ks-appearance-switch" aria-label="预览配色">
              {(['dark', 'light'] as Appearance[]).map((appearance) => (
                <button class={{ active: props.appearance === appearance }} onClick={() => emit('appearanceChange', appearance)}>
                  {appearance === 'dark' ? '深色' : '浅色'}
                </button>
              ))}
            </div>
          </div>
          {props.mode.startsWith('window') && <div class="ks-style-section ks-card-editor">
            <CardStylePreview document={props.effectiveDocument} mode={props.mode} appearance={props.appearance} />
            <div class="ks-card-editor-controls">
              <strong>颜色、透明度与边框</strong>
              {renderFields(modeFields.value.colors.filter(isCardField))}
              <strong>字体、尺寸与间距</strong>
              {renderFields(modeFields.value.advanced.filter(isCardField))}
            </div>
            <strong>位置与排列</strong>
            {renderFields(modeFields.value.layout.filter(isCardField))}
            {['window', 'window_editor'].includes(props.mode) && <CardPositionEditor
              position={getConfigPath(props.effectiveDocument, positionRoot.value + '.position') as string[] ?? ['50%', '50%', '50%', '50%']}
              reference={String(getConfigPath(props.effectiveDocument, positionRoot.value + '.position_mode') ?? 'window')}
              onChange={value => update(positionRoot.value + '.position', value)} />}
          </div>}
          <div class="ks-style-section">
            <span class="ks-style-section-label">{props.appearance === 'dark' ? '深色主题' : '浅色主题'}</span>
            {renderFields(palette)}
          </div>
          <div class="ks-style-section">
            <span class="ks-style-section-label">模式颜色</span>
            {renderFields(modeFields.value.colors.filter(field => !isCardField(field)))}
          </div>
          <div class="ks-style-section">
            <span class="ks-style-section-label">常用布局</span>
            {renderFields(modeFields.value.layout.filter(field => !isCardField(field)))}
          </div>
          <details class="ks-style-advanced">
            <summary>高级样式</summary>
            {renderFields(modeFields.value.advanced.filter(field => !isCardField(field)))}
          </details>
        </div>
      )
    }
  },
})

const StyleControl = defineComponent({
  props: {
    field: { type: Object as () => StyleField, required: true },
    value: { required: false },
    appearance: { type: String as () => Appearance, required: true },
    inherited: { type: Boolean, required: true },
    onUpdate: { type: Function as unknown as () => (value: unknown) => void, required: true },
    onReset: { type: Function as unknown as () => () => void, required: true },
  },
  setup(props) {
    return () => {
      const field = props.field
      const source = props.value ?? fallback(field, props.appearance)
      const variants = field.kind === 'color' && source && typeof source === 'object' ? source as Record<string, string> : undefined
      const value = variants?.[props.appearance] ?? source
      const updateColor = (next: string) => props.onUpdate(variants ? { ...variants, [props.appearance]: next } : next)
      return (
        <label class={{ 'ks-style-control': true, toggle: field.kind === 'boolean' }} title={field.path}>
          <span>{field.label}</span>
          {field.kind === 'boolean' ? (
            <button type="button" class={{ active: Boolean(value) }} onClick={() => props.onUpdate(!value)}><i />{value ? '开启' : '关闭'}</button>
          ) : field.kind === 'color' ? (
            <div class="ks-style-color">
              <input type="color" value={normalizeColor(value, props.appearance)} onInput={(event) => updateColor(withAlpha((event.target as HTMLInputElement).value, value))} />
              <input value={String(value)} onInput={(event) => updateColor((event.target as HTMLInputElement).value)} />
              <input class="ks-color-alpha" type="range" aria-label={`${field.label}不透明度`} title="不透明度" min="0" max="255"
                value={/^#[0-9a-f]{8}$/i.test(String(value)) ? parseInt(String(value).slice(7), 16) : 255}
                onInput={event => updateColor(`${normalizeColor(value, props.appearance)}${Number((event.target as HTMLInputElement).value).toString(16).padStart(2, '0')}`)} />
            </div>
          ) : field.kind === 'percentages' ? (
            <input value={Array.isArray(value) ? value.join(', ') : String(value)} onChange={event => {
              const input = event.target as HTMLInputElement
              const values = input.value.split(',').map(part => part.trim())
              try { cardPositionRatios(values); input.setCustomValidity(''); props.onUpdate(values) }
              catch (error) { input.setCustomValidity(String(error)); input.reportValidity() }
            }} />
          ) : field.kind === 'ratios' ? (
            <input value={Array.isArray(value) ? value.join(', ') : String(value)} onChange={event => {
              const input = event.target as HTMLInputElement
              const values = input.value.split(',').map(part => part.trim()).map(part => part.includes('/') ? part : Number(part))
              try { parseSplitRatios(values); input.setCustomValidity(''); props.onUpdate(values) }
              catch (error) { input.setCustomValidity(String(error)); input.reportValidity() }
            }} />
          ) : field.kind === 'select' ? (
            <select value={String(value)} onChange={(event) => props.onUpdate((event.target as HTMLSelectElement).value)}>
              {field.options?.map((option) => <option value={option}>{option}</option>)}
            </select>
          ) : (
            <input
              type={field.kind === 'number' ? 'number' : 'text'}
              value={String(value)}
              min={field.min}
              max={field.max}
              step={field.step}
              onInput={(event) => props.onUpdate(field.kind === 'number' ? Number((event.target as HTMLInputElement).value) : (event.target as HTMLInputElement).value)}
            />
          )}
          <button
            type="button"
            class={{ 'ks-style-reset': true, inherited: props.inherited }}
            disabled={props.inherited}
            onClick={(event) => { event.preventDefault(); props.onReset() }}
          >{props.inherited ? '默认' : '重置'}</button>
        </label>
      )
    }
  },
})

function fallback(field: StyleField, appearance: Appearance): unknown {
  if (field.kind === 'boolean') return false
  if (field.kind === 'number') return field.min === -1 ? -1 : field.min ?? 0
  if (field.kind === 'color') return appearance === 'light' ? '#6477D4FF' : '#6E82D6FF'
  return field.options?.[0] ?? ''
}

function normalizeColor(value: unknown, appearance: Appearance): string {
  const fallback = appearance === 'light' ? '#6477D4' : '#6E82D6'
  const source = String(value ?? fallback)
  return /^#[0-9a-f]{6}/i.test(source) ? source.slice(0, 7) : fallback
}

function withAlpha(next: string, previous: unknown): string {
  const source = String(previous ?? '')
  return `${next.toUpperCase()}${/^#[0-9a-f]{8}$/i.test(source) ? source.slice(7, 9).toUpperCase() : 'FF'}`
}

function modeLabel(mode: TargetingMode): string {
  return mode.startsWith('window') ? mode : mode === 'key_help' ? '按键提示' : mode === 'grid' ? 'Grid' : mode === 'recursive_grid' ? 'Recursive Grid' : 'UI Hint'
}
