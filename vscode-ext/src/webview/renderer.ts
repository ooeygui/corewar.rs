import type { BattleState, CellUpdate, WarriorSnapshot } from './battleEngine';

interface RgbColor {
  r: number;
  g: number;
  b: number;
}

interface SelectionPayload {
  address: number;
}

export class CoreRenderer {
  private readonly canvas: HTMLCanvasElement;
  private readonly context: CanvasRenderingContext2D;
  private readonly bitmapCanvas: HTMLCanvasElement;
  private readonly bitmapContext: CanvasRenderingContext2D;
  private readonly heat = new Map<number, number>();
  private readonly dirtyAddresses = new Set<number>();
  private readonly palette = new Map<number, RgbColor>();
  private readonly processAddresses = new Set<number>();
  private state: BattleState | null = null;
  private zoom = 1;
  private minZoom = 1;
  private maxZoom = 40;
  private panX = 0;
  private panY = 0;
  private cols = 1;
  private rows = 1;
  private needsPresent = true;
  private hasFittedViewport = false;
  private dragStartX = 0;
  private dragStartY = 0;
  private originPanX = 0;
  private originPanY = 0;
  private isDragging = false;
  private dragDistance = 0;
  private selectedAddress: number | null = null;
  private selectionListener?: (payload: SelectionPayload) => void;

  public constructor(canvas: HTMLCanvasElement) {
    const context = canvas.getContext('2d');
    const bitmapCanvas = document.createElement('canvas');
    const bitmapContext = bitmapCanvas.getContext('2d');

    if (context === null || bitmapContext === null) {
      throw new Error('Canvas 2D rendering is not available.');
    }

    this.canvas = canvas;
    this.context = context;
    this.bitmapCanvas = bitmapCanvas;
    this.bitmapContext = bitmapContext;

    this.context.imageSmoothingEnabled = false;
    this.bitmapContext.imageSmoothingEnabled = false;

    this.attachEventListeners();
    this.resize();
  }

  public onSelect(listener: (payload: SelectionPayload) => void): void {
    this.selectionListener = listener;
  }

  public setState(state: BattleState): void {
    this.state = state;
    this.cols = Math.ceil(Math.sqrt(state.core.length));
    this.rows = Math.ceil(state.core.length / this.cols);
    this.bitmapCanvas.width = this.cols;
    this.bitmapCanvas.height = this.rows;
    this.palette.clear();
    state.warriors.forEach((warrior) => {
      this.palette.set(warrior.id, this.parseColor(warrior.color));
    });
    this.syncProcesses(state.warriors);
    this.markAllDirty();

    if (!this.hasFittedViewport) {
      this.fitToView();
      this.hasFittedViewport = true;
    }

    this.needsPresent = true;
  }

  public applyUpdates(updates: CellUpdate[]): void {
    if (this.state === null) {
      return;
    }

    updates.forEach((update) => {
      this.state!.core[update.address] = update.cell;
      this.dirtyAddresses.add(update.address);
      this.boostHeat(update.address, 1);
    });
    this.needsPresent = true;
  }

  public markExecution(address: number): void {
    this.boostHeat(address, 0.9);
    this.dirtyAddresses.add(address);
    this.needsPresent = true;
  }

  public updateWarriors(warriors: WarriorSnapshot[]): void {
    this.syncProcesses(warriors);
    this.needsPresent = true;
  }

  public resize(): void {
    const parent = this.canvas.parentElement;
    const width = parent?.clientWidth ?? this.canvas.clientWidth;
    const height = parent?.clientHeight ?? this.canvas.clientHeight;
    const ratio = window.devicePixelRatio || 1;

    this.canvas.width = Math.max(1, Math.floor(width * ratio));
    this.canvas.height = Math.max(1, Math.floor(height * ratio));
    this.canvas.style.width = `${Math.max(1, width)}px`;
    this.canvas.style.height = `${Math.max(1, height)}px`;
    this.context.setTransform(1, 0, 0, 1, 0, 0);
    this.context.scale(ratio, ratio);
    this.needsPresent = true;

    if (this.state !== null && !this.isDragging) {
      this.fitToView();
    }
  }

  public fitToView(): void {
    if (this.state === null) {
      return;
    }

    const width = this.canvas.clientWidth || 1;
    const height = this.canvas.clientHeight || 1;
    this.minZoom = Math.max(1, Math.min(width / this.cols, height / this.rows));
    this.zoom = Math.max(this.minZoom, this.zoom);
    this.panX = (width - this.cols * this.zoom) / 2;
    this.panY = (height - this.rows * this.zoom) / 2;
    this.needsPresent = true;
  }

  public render(): void {
    if (this.state === null) {
      this.drawEmptyState();
      return;
    }

    this.decayHeat();
    if (!this.needsPresent && this.dirtyAddresses.size === 0) {
      return;
    }

    this.flushDirtyCells();

    const width = this.canvas.clientWidth || 1;
    const height = this.canvas.clientHeight || 1;
    this.context.clearRect(0, 0, width, height);
    this.context.fillStyle = this.getCssVariable('--vscode-editor-background', '#1e1e1e');
    this.context.fillRect(0, 0, width, height);
    this.context.drawImage(
      this.bitmapCanvas,
      this.panX,
      this.panY,
      this.cols * this.zoom,
      this.rows * this.zoom
    );

    this.drawProcessMarkers();
    this.drawSelection();
    this.needsPresent = false;
  }

  private drawEmptyState(): void {
    const width = this.canvas.clientWidth || 1;
    const height = this.canvas.clientHeight || 1;
    this.context.clearRect(0, 0, width, height);
    this.context.fillStyle = this.getCssVariable('--vscode-editor-background', '#1e1e1e');
    this.context.fillRect(0, 0, width, height);
    this.context.fillStyle = this.getCssVariable('--vscode-descriptionForeground', '#9da1a6');
    this.context.font = '13px sans-serif';
    this.context.fillText('Load a warrior to start the battle view.', 16, 28);
  }

  private drawProcessMarkers(): void {
    if (this.processAddresses.size === 0) {
      return;
    }

    this.context.save();
    this.context.strokeStyle = 'rgba(255, 255, 255, 0.9)';
    this.context.lineWidth = Math.max(1, Math.min(2, this.zoom / 4));

    this.processAddresses.forEach((address) => {
      const point = this.addressToPoint(address);
      const x = this.panX + point.x * this.zoom;
      const y = this.panY + point.y * this.zoom;
      this.context.strokeRect(x, y, Math.max(this.zoom, 1), Math.max(this.zoom, 1));
    });

    this.context.restore();
  }

  private drawSelection(): void {
    if (this.selectedAddress === null) {
      return;
    }

    const point = this.addressToPoint(this.selectedAddress);
    const x = this.panX + point.x * this.zoom;
    const y = this.panY + point.y * this.zoom;

    this.context.save();
    this.context.strokeStyle = this.getCssVariable('--vscode-focusBorder', '#3794ff');
    this.context.lineWidth = Math.max(1, Math.min(3, this.zoom / 3));
    this.context.strokeRect(x - 1, y - 1, Math.max(this.zoom, 1) + 2, Math.max(this.zoom, 1) + 2);
    this.context.restore();
  }

  private flushDirtyCells(): void {
    this.dirtyAddresses.forEach((address) => {
      const point = this.addressToPoint(address);
      const cell = this.state?.core[address];
      const color = this.getCellColor(cell?.ownerId ?? null, this.heat.get(address) ?? 0);
      this.bitmapContext.fillStyle = `rgb(${color.r}, ${color.g}, ${color.b})`;
      this.bitmapContext.fillRect(point.x, point.y, 1, 1);
    });
    this.dirtyAddresses.clear();
  }

  private getCellColor(ownerId: number | null, heat: number): RgbColor {
    const base = ownerId === null
      ? this.parseColor(this.getCssVariable('--vscode-editor-background', '#1e1e1e'))
      : this.palette.get(ownerId) ?? this.parseColor('hsl(0deg 0% 50%)');
    const glow = Math.min(1, heat);

    return {
      r: Math.min(255, Math.round(base.r + (255 - base.r) * glow * 0.45)),
      g: Math.min(255, Math.round(base.g + (255 - base.g) * glow * 0.45)),
      b: Math.min(255, Math.round(base.b + (255 - base.b) * glow * 0.45))
    };
  }

  private decayHeat(): void {
    let dirty = false;
    this.heat.forEach((value, address) => {
      const next = value * 0.92;
      if (next < 0.025) {
        this.heat.delete(address);
        this.dirtyAddresses.add(address);
        dirty = true;
        return;
      }
      if (Math.abs(next - value) > 0.02) {
        this.heat.set(address, next);
        this.dirtyAddresses.add(address);
        dirty = true;
      }
    });
    if (dirty) {
      this.needsPresent = true;
    }
  }

  private boostHeat(address: number, amount: number): void {
    const next = Math.min(1, Math.max(this.heat.get(address) ?? 0, amount));
    this.heat.set(address, next);
  }

  private markAllDirty(): void {
    if (this.state === null) {
      return;
    }
    for (let address = 0; address < this.state.core.length; address += 1) {
      this.dirtyAddresses.add(address);
    }
  }

  private syncProcesses(warriors: WarriorSnapshot[]): void {
    this.processAddresses.clear();
    warriors.forEach((warrior) => {
      warrior.processes.forEach((address) => {
        this.processAddresses.add(address);
      });
    });
  }

  private addressToPoint(address: number): { x: number; y: number } {
    return {
      x: address % this.cols,
      y: Math.floor(address / this.cols)
    };
  }

  private screenToAddress(clientX: number, clientY: number): number | null {
    if (this.state === null) {
      return null;
    }

    const rect = this.canvas.getBoundingClientRect();
    const x = (clientX - rect.left - this.panX) / this.zoom;
    const y = (clientY - rect.top - this.panY) / this.zoom;
    const gridX = Math.floor(x);
    const gridY = Math.floor(y);

    if (gridX < 0 || gridY < 0 || gridX >= this.cols || gridY >= this.rows) {
      return null;
    }

    const address = gridY * this.cols + gridX;
    return address < this.state.core.length ? address : null;
  }

  private attachEventListeners(): void {
    this.canvas.addEventListener('mousedown', (event) => {
      this.isDragging = true;
      this.dragDistance = 0;
      this.dragStartX = event.clientX;
      this.dragStartY = event.clientY;
      this.originPanX = this.panX;
      this.originPanY = this.panY;
    });

    window.addEventListener('mousemove', (event) => {
      if (!this.isDragging) {
        return;
      }

      const deltaX = event.clientX - this.dragStartX;
      const deltaY = event.clientY - this.dragStartY;
      this.dragDistance = Math.max(this.dragDistance, Math.abs(deltaX) + Math.abs(deltaY));
      this.panX = this.originPanX + deltaX;
      this.panY = this.originPanY + deltaY;
      this.needsPresent = true;
    });

    window.addEventListener('mouseup', (event) => {
      if (!this.isDragging) {
        return;
      }
      this.isDragging = false;
      if (this.dragDistance < 4) {
        const address = this.screenToAddress(event.clientX, event.clientY);
        if (address !== null) {
          this.selectedAddress = address;
          this.selectionListener?.({ address });
          this.needsPresent = true;
        }
      }
    });

    this.canvas.addEventListener('wheel', (event) => {
      event.preventDefault();
      const rect = this.canvas.getBoundingClientRect();
      const cursorX = event.clientX - rect.left;
      const cursorY = event.clientY - rect.top;
      const worldX = (cursorX - this.panX) / this.zoom;
      const worldY = (cursorY - this.panY) / this.zoom;
      const factor = event.deltaY < 0 ? 1.12 : 0.89;
      this.zoom = Math.min(this.maxZoom, Math.max(this.minZoom, this.zoom * factor));
      this.panX = cursorX - worldX * this.zoom;
      this.panY = cursorY - worldY * this.zoom;
      this.needsPresent = true;
    }, { passive: false });

    window.addEventListener('resize', () => this.resize());
  }

  private parseColor(color: string): RgbColor {
    const probe = document.createElement('span');
    probe.style.color = color;
    document.body.appendChild(probe);
    const resolved = getComputedStyle(probe).color;
    probe.remove();
    const match = resolved.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)/i);
    if (match === null) {
      return { r: 128, g: 128, b: 128 };
    }
    return {
      r: Number.parseInt(match[1], 10),
      g: Number.parseInt(match[2], 10),
      b: Number.parseInt(match[3], 10)
    };
  }

  private getCssVariable(name: string, fallback: string): string {
    const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
    return value || fallback;
  }
}
