import { BattleEngine, type BattleCompleteResult } from './battleEngine';
import { CoreRenderer } from './renderer';

declare function acquireVsCodeApi(): { postMessage(message: unknown): void };

type HostMessage =
  | { type: 'loadWarrior'; source: string; name: string }
  | { type: 'play' }
  | { type: 'pause' }
  | { type: 'step' }
  | { type: 'setSpeed'; speed: number }
  | { type: 'reset' };

const vscode = acquireVsCodeApi();
const engine = new BattleEngine();
const canvas = document.getElementById('core-canvas');
const addWarriorButton = document.getElementById('add-warrior-button');
const playPauseButton = document.getElementById('play-pause-button');
const stepButton = document.getElementById('step-button');
const resetButton = document.getElementById('reset-button');
const speedSlider = document.getElementById('speed-slider');
const speedValue = document.getElementById('speed-value');
const cycleCounter = document.getElementById('cycle-counter');
const warriorList = document.getElementById('warrior-list');
const inspector = document.getElementById('cell-inspector');

if (
  !(canvas instanceof HTMLCanvasElement) ||
  !(addWarriorButton instanceof HTMLButtonElement) ||
  !(playPauseButton instanceof HTMLButtonElement) ||
  !(stepButton instanceof HTMLButtonElement) ||
  !(resetButton instanceof HTMLButtonElement) ||
  !(speedSlider instanceof HTMLInputElement) ||
  !(speedValue instanceof HTMLElement) ||
  !(cycleCounter instanceof HTMLElement) ||
  !(warriorList instanceof HTMLElement) ||
  !(inspector instanceof HTMLElement)
) {
  throw new Error('Battle view UI failed to initialize.');
}

const renderer = new CoreRenderer(canvas);
let isPlaying = false;
let cycleBudget = 0;
let selectedAddress: number | null = null;

const refreshUi = (): void => {
  const summary = engine.getSummary();
  cycleCounter.textContent = summary.cycle.toString();
  playPauseButton.textContent = isPlaying ? 'Pause ⏸' : 'Play ▶';
  renderWarriorList(summary.warriors);
  renderer.updateWarriors(summary.warriors);
  renderInspector();
};

const renderWarriorList = (warriors: ReturnType<typeof engine.getSummary>['warriors']): void => {
  warriorList.replaceChildren();

  if (warriors.length === 0) {
    const emptyState = document.createElement('li');
    emptyState.className = 'warrior-empty';
    emptyState.textContent = 'No warriors loaded.';
    warriorList.appendChild(emptyState);
    return;
  }

  warriors.forEach((warrior) => {
    const item = document.createElement('li');
    item.className = 'warrior-item';

    const swatch = document.createElement('span');
    swatch.className = 'warrior-swatch';
    swatch.style.background = warrior.color;

    const details = document.createElement('div');
    details.className = 'warrior-details';

    const name = document.createElement('span');
    name.className = 'warrior-name';
    name.textContent = warrior.name;

    const meta = document.createElement('span');
    meta.className = 'warrior-meta';
    meta.textContent = `${warrior.processCount} process${warrior.processCount === 1 ? '' : 'es'}${warrior.alive ? '' : ' • dead'}`;

    details.append(name, meta);

    const removeButton = document.createElement('button');
    removeButton.className = 'warrior-remove';
    removeButton.type = 'button';
    removeButton.textContent = 'Remove';
    removeButton.addEventListener('click', () => {
      engine.removeWarrior(warrior.id);
      renderer.setState(engine.getState());
      refreshUi();
    });

    item.append(swatch, details, removeButton);
    warriorList.appendChild(item);
  });
};

const renderInspector = (): void => {
  if (selectedAddress === null) {
    inspector.textContent = 'Click a core cell to inspect its instruction.';
    return;
  }

  const state = engine.getState();
  const cell = state.core[selectedAddress];
  const owner = state.warriors.find((warrior) => warrior.id === cell.ownerId);
  inspector.innerHTML = '';

  const title = document.createElement('div');
  title.className = 'inspector-title';
  title.textContent = `Address ${selectedAddress}`;

  const ownerLine = document.createElement('div');
  ownerLine.className = 'inspector-line';
  ownerLine.textContent = `Owner: ${owner?.name ?? 'None'}`;

  const instructionLine = document.createElement('pre');
  instructionLine.className = 'inspector-code';
  instructionLine.textContent = `${cell.instruction.opcode}.${cell.instruction.modifier} ${cell.instruction.a.mode}${cell.instruction.a.value}, ${cell.instruction.b.mode}${cell.instruction.b.value}`;

  inspector.append(title, ownerLine, instructionLine);
};

const setSpeed = (speed: number): void => {
  const clamped = Math.max(1, Math.min(1000, Math.round(speed)));
  speedSlider.value = clamped.toString();
  speedValue.textContent = `${clamped}x`;
};

const stopPlayback = (): void => {
  isPlaying = false;
  cycleBudget = 0;
  refreshUi();
};

const handleBattleComplete = (result: BattleCompleteResult): void => {
  stopPlayback();
  vscode.postMessage({ type: 'battleComplete', result });
};

engine.on('reset', (state) => {
  renderer.setState(state);
  refreshUi();
});
engine.on('memoryWrite', (payload) => renderer.applyUpdates(payload.updates));
engine.on('execute', (payload) => renderer.markExecution(payload.address));
engine.on('processChange', () => refreshUi());
engine.on('battleComplete', handleBattleComplete);
renderer.onSelect(({ address }) => {
  selectedAddress = address;
  renderInspector();
});

addWarriorButton.addEventListener('click', () => {
  vscode.postMessage({ type: 'pickWarrior' });
});
playPauseButton.addEventListener('click', () => {
  isPlaying = !isPlaying;
  refreshUi();
});
stepButton.addEventListener('click', () => {
  engine.step();
  refreshUi();
});
resetButton.addEventListener('click', () => {
  stopPlayback();
  engine.reset();
});
speedSlider.addEventListener('input', () => {
  setSpeed(Number.parseInt(speedSlider.value, 10));
});

window.addEventListener('message', (event: MessageEvent<HostMessage>) => {
  const message = event.data;
  try {
    switch (message.type) {
      case 'loadWarrior':
        engine.loadWarrior(message.source, message.name);
        renderer.setState(engine.getState());
        refreshUi();
        break;
      case 'play':
        isPlaying = true;
        refreshUi();
        break;
      case 'pause':
        stopPlayback();
        break;
      case 'step':
        engine.step();
        refreshUi();
        break;
      case 'setSpeed':
        setSpeed(message.speed);
        break;
      case 'reset':
        stopPlayback();
        engine.reset();
        break;
      default:
        break;
    }
  } catch (error) {
    const messageText = error instanceof Error ? error.message : String(error);
    vscode.postMessage({ type: 'error', msg: messageText });
  }
});

const animate = (): void => {
  if (isPlaying) {
    const speed = Number.parseInt(speedSlider.value, 10);
    cycleBudget += speed / 60;
    let executed = 0;
    while (cycleBudget >= 1 && executed < 500 && !engine.getSummary().complete) {
      engine.step();
      cycleBudget -= 1;
      executed += 1;
    }
    if (executed > 0) {
      refreshUi();
    }
  }

  renderer.render();
  requestAnimationFrame(animate);
};

setSpeed(60);
refreshUi();
vscode.postMessage({ type: 'ready' });
requestAnimationFrame(animate);
