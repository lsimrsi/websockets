import { writable, type Writable } from "svelte/store";
import type { ChatMessage, Toast, ToastItem } from "./interfaces";
import { v4 as uuidv4 } from "uuid";

export const socket: Writable<WebSocket | null> = writable(null);
export const messages: Writable<ChatMessage[]> = writable([]);
export const name: Writable<string> = writable("");
export const hasRegisteredName: Writable<boolean> = writable(false);

function createToasts() {
  const { subscribe, set, update } = writable<ToastItem[]>([]);

  return {
    subscribe,
    add: (toast: ToastItem) => update((toasts: ToastItem[]) => {
      toasts = [...toasts, toast];
      return toasts;
    }),
    remove: (toast: ToastItem) => update(toasts => {
      let filteredToasts = toasts.filter(
        (ti: ToastItem) => ti.uuid !== toast.uuid
      );
      return [...filteredToasts];
    }),
    reset: () => set([])
  };
}

export const toasts = createToasts();



function createCount() {
  const { subscribe, set, update } = writable<number>(0);

  return {
    subscribe,
    increment: () => update(n => n + 1),
    decrement: () => update(n => n - 1),
    reset: () => set(0)
  };
}

export const count = createCount();