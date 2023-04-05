import { writable, type Writable } from "svelte/store";
import type { ChatMessage } from "./interfaces";

export const socket: Writable<WebSocket | null> = writable(null);
export const messages: Writable<ChatMessage[]> = writable([]);
export const name: Writable<string> = writable("");
export const hasRegisteredName: Writable<boolean> = writable(false);