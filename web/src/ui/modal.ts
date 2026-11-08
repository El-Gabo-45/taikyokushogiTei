/**
 * Minimal modal dialog used for move disambiguation (promotion / igui).
 *
 * Built from DOM nodes (never `innerHTML` with data) and resolves a promise
 * with the chosen value, or `null` when the user cancels.
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
  return new Promise<T | null>((resolve) => {
    const overlay = document.createElement('div');
    overlay.className = 'modal-overlay';

    const box = document.createElement('div');
    box.className = 'modal-box';
    box.setAttribute('role', 'dialog');
    box.setAttribute('aria-modal', 'true');

    const heading = document.createElement('h2');
    heading.textContent = title;
    box.appendChild(heading);

    if (description) {
      const paragraph = document.createElement('p');
      paragraph.className = 'modal-description';
      paragraph.textContent = description;
      box.appendChild(paragraph);
    }

    const list = document.createElement('div');
    list.className = 'modal-options';

    const close = (value: T | null) => {
      document.removeEventListener('keydown', onKey);
      overlay.remove();
      resolve(value);
    };

    options.forEach((option, index) => {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'modal-option';
      button.dataset.index = String(index);

      const label = document.createElement('span');
      label.className = 'modal-option-label';
      label.textContent = option.label;
      button.appendChild(label);

      if (option.detail) {
        const detail = document.createElement('span');
        detail.className = 'modal-option-detail';
        detail.textContent = option.detail;
        button.appendChild(detail);
      }

      button.addEventListener('click', () => close(option.value));
      list.appendChild(button);
    });
    box.appendChild(list);

    const cancel = document.createElement('button');
    cancel.type = 'button';
    cancel.className = 'modal-cancel';
    cancel.textContent = 'Cancel (Esc)';
    cancel.addEventListener('click', () => close(null));
    box.appendChild(cancel);

    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        close(null);
      }
    };
    document.addEventListener('keydown', onKey);
    overlay.addEventListener('click', (event) => {
      if (event.target === overlay) close(null);
    });

    overlay.appendChild(box);
    document.body.appendChild(overlay);
    const first = box.querySelector<HTMLButtonElement>('.modal-option');
    first?.focus();
  });
}
