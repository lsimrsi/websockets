import { writable, type Writable } from "svelte/store";

export const socket: Writable<WebSocket | null> = writable(null);