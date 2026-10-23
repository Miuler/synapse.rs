import { createDiv, createEl, empty } from './dom';
import type { SynapseApp } from './app';

/**
 * Clase base para componentes de control en Settings de Obsidian.
 */
export abstract class BaseComponent {
  disabled = false;

  setDisabled(disabled: boolean): this {
    this.disabled = disabled;
    return this;
  }
}

/**
 * Componente que gestiona un valor con evento de cambio `onChange`.
 */
export abstract class ValueComponent<T> extends BaseComponent {
  abstract getValue(): T;
  abstract setValue(value: T): this;
  abstract onChange(callback: (value: T) => any): this;
}

/** Componente de campo de texto input */
export class TextComponent extends ValueComponent<string> {
  readonly inputEl: HTMLInputElement;
  private changeCallback?: (val: string) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.inputEl = createEl('input', { parent: containerEl, attr: { type: 'text' } });
    this.inputEl.classList.add('setting-input-text');
    this.inputEl.addEventListener('input', () => {
      this.changeCallback?.(this.inputEl.value);
    });
  }

  getValue(): string {
    return this.inputEl.value;
  }

  setValue(value: string): this {
    this.inputEl.value = value ?? '';
    return this;
  }

  setPlaceholder(placeholder: string): this {
    this.inputEl.placeholder = placeholder;
    return this;
  }

  onChange(callback: (value: string) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.inputEl.disabled = disabled;
    return this;
  }
}

/** Componente de área de texto multilínea */
export class TextAreaComponent extends ValueComponent<string> {
  readonly inputEl: HTMLTextAreaElement;
  private changeCallback?: (val: string) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.inputEl = createEl('textarea', { parent: containerEl });
    this.inputEl.classList.add('setting-textarea');
    this.inputEl.addEventListener('input', () => {
      this.changeCallback?.(this.inputEl.value);
    });
  }

  getValue(): string {
    return this.inputEl.value;
  }

  setValue(value: string): this {
    this.inputEl.value = value ?? '';
    return this;
  }

  setPlaceholder(placeholder: string): this {
    this.inputEl.placeholder = placeholder;
    return this;
  }

  onChange(callback: (value: string) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.inputEl.disabled = disabled;
    return this;
  }
}

/** Componente de interruptor Toggle (booleano) */
export class ToggleComponent extends ValueComponent<boolean> {
  readonly toggleEl: HTMLElement;
  private value = false;
  private changeCallback?: (val: boolean) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.toggleEl = createDiv({ cls: 'checkbox-container', parent: containerEl });
    this.toggleEl.setAttribute('role', 'switch');
    this.toggleEl.setAttribute('aria-checked', 'false');

    this.toggleEl.addEventListener('click', () => {
      if (this.disabled) return;
      this.setValue(!this.value);
      this.changeCallback?.(this.value);
    });
  }

  getValue(): boolean {
    return this.value;
  }

  setValue(value: boolean): this {
    this.value = Boolean(value);
    if (this.value) {
      this.toggleEl.classList.add('is-enabled');
      this.toggleEl.setAttribute('aria-checked', 'true');
    } else {
      this.toggleEl.classList.remove('is-enabled');
      this.toggleEl.setAttribute('aria-checked', 'false');
    }
    return this;
  }

  onChange(callback: (value: boolean) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    if (disabled) {
      this.toggleEl.classList.add('is-disabled');
    } else {
      this.toggleEl.classList.remove('is-disabled');
    }
    return this;
  }
}

/** Componente de menú desplegable Dropdown */
export class DropdownComponent extends ValueComponent<string> {
  readonly selectEl: HTMLSelectElement;
  private changeCallback?: (val: string) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.selectEl = createEl('select', { parent: containerEl, cls: 'dropdown' });
    this.selectEl.addEventListener('change', () => {
      this.changeCallback?.(this.selectEl.value);
    });
  }

  addOption(value: string, display: string): this {
    const opt = createEl('option', { parent: this.selectEl, text: display, attr: { value } });
    return this;
  }

  addOptions(options: Record<string, string>): this {
    for (const [val, display] of Object.entries(options)) {
      this.addOption(val, display);
    }
    return this;
  }

  getValue(): string {
    return this.selectEl.value;
  }

  setValue(value: string): this {
    this.selectEl.value = value;
    return this;
  }

  onChange(callback: (value: string) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.selectEl.disabled = disabled;
    return this;
  }
}

/** Componente de control deslizante Slider */
export class SliderComponent extends ValueComponent<number> {
  readonly sliderEl: HTMLInputElement;
  private changeCallback?: (val: number) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.sliderEl = createEl('input', { parent: containerEl, cls: 'slider', attr: { type: 'range' } });
    this.sliderEl.addEventListener('input', () => {
      this.changeCallback?.(this.getValue());
    });
  }

  setLimits(min: number, max: number, step: number): this {
    this.sliderEl.min = String(min);
    this.sliderEl.max = String(max);
    this.sliderEl.step = String(step);
    return this;
  }

  setDynamicTooltip(): this {
    return this;
  }

  getValue(): number {
    return Number(this.sliderEl.value);
  }

  setValue(value: number): this {
    this.sliderEl.value = String(value);
    return this;
  }

  onChange(callback: (value: number) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.sliderEl.disabled = disabled;
    return this;
  }
}

/** Componente de botón Button */
export class ButtonComponent extends BaseComponent {
  readonly buttonEl: HTMLButtonElement;

  constructor(containerEl: HTMLElement) {
    super();
    this.buttonEl = createEl('button', { parent: containerEl, cls: 'mod-cta' });
  }

  setButtonText(text: string): this {
    this.buttonEl.textContent = text;
    return this;
  }

  setIcon(_icon: string): this {
    return this;
  }

  setCta(): this {
    this.buttonEl.classList.add('mod-cta');
    return this;
  }

  setWarning(): this {
    this.buttonEl.classList.add('mod-warning');
    return this;
  }

  setTooltip(tooltip: string): this {
    this.buttonEl.title = tooltip;
    return this;
  }

  onClick(callback: (evt: MouseEvent) => any): this {
    this.buttonEl.addEventListener('click', callback as any);
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.buttonEl.disabled = disabled;
    return this;
  }
}

/** Componente de botón adicional ligero ExtraButton */
export class ExtraButtonComponent extends BaseComponent {
  readonly extraSettingsEl: HTMLElement;

  constructor(containerEl: HTMLElement) {
    super();
    this.extraSettingsEl = createDiv({ cls: 'clickable-icon extra-setting-button', parent: containerEl });
  }

  setIcon(_icon: string): this {
    return this;
  }

  setTooltip(tooltip: string): this {
    this.extraSettingsEl.title = tooltip;
    return this;
  }

  onClick(callback: (evt: MouseEvent) => any): this {
    this.extraSettingsEl.addEventListener('click', callback as any);
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    if (disabled) {
      this.extraSettingsEl.classList.add('is-disabled');
    } else {
      this.extraSettingsEl.classList.remove('is-disabled');
    }
    return this;
  }
}

/** Componente de selección de color ColorPicker */
export class ColorComponent extends ValueComponent<string> {
  readonly inputEl: HTMLInputElement;
  private changeCallback?: (val: string) => any;

  constructor(containerEl: HTMLElement) {
    super();
    this.inputEl = createEl('input', { parent: containerEl, cls: 'color-picker', attr: { type: 'color' } });
    this.inputEl.addEventListener('input', () => {
      this.changeCallback?.(this.inputEl.value);
    });
  }

  getValue(): string {
    return this.inputEl.value;
  }

  setValue(value: string): this {
    this.inputEl.value = value;
    return this;
  }

  onChange(callback: (value: string) => any): this {
    this.changeCallback = callback;
    return this;
  }

  override setDisabled(disabled: boolean): this {
    super.setDisabled(disabled);
    this.inputEl.disabled = disabled;
    return this;
  }
}

/**
 * Constructor fluido Setting de Obsidian.
 * Representa una fila en el diálogo de configuración con título, descripción y controles.
 */
export class Setting {
  readonly settingEl: HTMLElement;
  readonly infoEl: HTMLElement;
  readonly nameEl: HTMLElement;
  readonly descEl: HTMLElement;
  readonly controlEl: HTMLElement;

  constructor(containerEl: HTMLElement) {
    this.settingEl = createDiv({ cls: 'setting-item', parent: containerEl });
    this.infoEl = createDiv({ cls: 'setting-item-info', parent: this.settingEl });
    this.nameEl = createDiv({ cls: 'setting-item-name', parent: this.infoEl });
    this.descEl = createDiv({ cls: 'setting-item-description', parent: this.infoEl });
    this.controlEl = createDiv({ cls: 'setting-item-control', parent: this.settingEl });
  }

  setName(name: string | DocumentFragment): this {
    if (typeof name === 'string') {
      this.nameEl.textContent = name;
    } else if (name && (name as any).cloneNode) {
      empty(this.nameEl);
      this.nameEl.appendChild(name);
    }
    return this;
  }

  setDesc(desc: string | DocumentFragment): this {
    if (typeof desc === 'string') {
      this.descEl.textContent = desc;
    } else if (desc && (desc as any).cloneNode) {
      empty(this.descEl);
      this.descEl.appendChild(desc);
    }
    return this;
  }

  setClass(cls: string): this {
    this.settingEl.classList.add(cls);
    return this;
  }

  setTooltip(tooltip: string): this {
    this.settingEl.title = tooltip;
    return this;
  }

  setHeading(): this {
    this.settingEl.classList.add('setting-item-heading');
    return this;
  }

  setDisabled(disabled: boolean): this {
    if (disabled) {
      this.settingEl.classList.add('is-disabled');
    } else {
      this.settingEl.classList.remove('is-disabled');
    }
    return this;
  }

  addText(cb: (text: TextComponent) => any): this {
    const text = new TextComponent(this.controlEl);
    cb(text);
    return this;
  }

  addTextArea(cb: (textArea: TextAreaComponent) => any): this {
    const textArea = new TextAreaComponent(this.controlEl);
    cb(textArea);
    return this;
  }

  addToggle(cb: (toggle: ToggleComponent) => any): this {
    const toggle = new ToggleComponent(this.controlEl);
    cb(toggle);
    return this;
  }

  addDropdown(cb: (dropdown: DropdownComponent) => any): this {
    const dropdown = new DropdownComponent(this.controlEl);
    cb(dropdown);
    return this;
  }

  addSlider(cb: (slider: SliderComponent) => any): this {
    const slider = new SliderComponent(this.controlEl);
    cb(slider);
    return this;
  }

  addButton(cb: (button: ButtonComponent) => any): this {
    const button = new ButtonComponent(this.controlEl);
    cb(button);
    return this;
  }

  addExtraButton(cb: (button: ExtraButtonComponent) => any): this {
    const button = new ExtraButtonComponent(this.controlEl);
    cb(button);
    return this;
  }

  addColorPicker(cb: (color: ColorComponent) => any): this {
    const color = new ColorComponent(this.controlEl);
    cb(color);
    return this;
  }
}

/**
 * Pestaña base para opciones de configuración de plugins (PluginSettingTab).
 */
export abstract class PluginSettingTab {
  readonly app: SynapseApp;
  readonly plugin: any;
  readonly containerEl: HTMLElement;

  constructor(app: SynapseApp, plugin: any) {
    this.app = app;
    this.plugin = plugin;
    this.containerEl = createDiv({ cls: 'vertical-tab-content plugin-settings-tab' });
  }

  /** Renderiza los controles de la pestaña en `containerEl` */
  abstract display(): void;

  /** Limpia el DOM de la pestaña al ocultarse */
  hide(): void {
    empty(this.containerEl);
  }
}

/**
 * Gestor central de configuración de SynapseApp (app.setting).
 */
export class SynapseSettingManager {
  readonly settingTabs: PluginSettingTab[] = [];
  activeTab: PluginSettingTab | null = null;

  addSettingTab(tab: PluginSettingTab): void {
    this.settingTabs.push(tab);
  }

  removeSettingTab(tab: PluginSettingTab): void {
    const idx = this.settingTabs.indexOf(tab);
    if (idx !== -1) {
      this.settingTabs.splice(idx, 1);
      if (this.activeTab === tab) {
        this.activeTab = null;
      }
    }
  }

  getSettingTab(pluginId: string): PluginSettingTab | undefined {
    return this.settingTabs.find((t) => t.plugin?.manifest?.id === pluginId);
  }

  openTab(tab: PluginSettingTab): void {
    if (this.activeTab && this.activeTab !== tab) {
      this.activeTab.hide();
    }
    this.activeTab = tab;
    tab.display();
  }
}
