/**
 * Move-disambiguation dialog using the native <dialog> element.
 *
 * The browser provides inert page, focus containment, top-layer rendering,
 * ::backdrop, Esc dismissal and focus return to the invoker — no custom
 * focus trap, role="dialog" or aria-modal needed (see modern-web-guidance
 * guides/accessibility §12 and guides/ui-components/dialog.md).
 *
 * Resolves a promise with the chosen value, or null when cancelled.
 */

export interface ModalChoice<T> {
  label: string;
  detail?: string;
  value: T;
}

export function openChoiceModal<T>(
  title: string,
  description: string,
  options: ModalChoice<T>[],
): Promise<T | null> {
  const dialog = document.getElementById('choice-dialog') as HTMLDialogElement | null;
  const titleEl = document.getElementById('choice-title') as HTMLElement | null;
  const descEl = document.getElementById('choice-description') as HTMLElement | null;
  const list = document.getElementById('choice-options') as HTMLElement | null;
  if (!dialog || !titleEl || !descEl || !list) {
    throw new Error('choice-dialog elements missing from index.html');
  }

  return new Promise<T | null>((resolve) => {
    titleEl.textContent = title;
    descEl.textContent = description;
    list.replaceChildren();

    // One controller owns every listener added for this modal opening, so a
    // second open() cannot accumulate stale handlers on the shared <dialog>.
    const listeners = new AbortController();
    const signal = listeners.signal;
    let settled = false;

    const close = (value: T | null) => {
      if (settled) return;
      settled = true;
      listeners.abort(); // drop button/cancel/close listeners atomically
      dialog.close();
      resolve(value);
      dialog.returnValue = '';
    };

    options.forEach((option, index) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.value = String(index);
      button.textContent = option.label;
      if (option.detail) {
        button.title = option.detail;
      }
      button.addEventListener('click', () => close(option.value), { signal });
      list.appendChild(button);
    });

    const cancel = dialog.querySelector<HTMLButtonElement>('.cancel-row')!;
    cancel.addEventListener('click', () => close(null), { signal });

    const onClose = () => {
      // Esc or other close request with no chosen value → cancelled.
      if (!settled) {
        settled = true;
        listeners.abort();
        resolve(null);
      }
    };
    dialog.addEventListener('close', onClose, { signal });

    dialog.showModal();
    const first = list.querySelector<HTMLButtonElement>('button');
    first?.focus();
  });
}
