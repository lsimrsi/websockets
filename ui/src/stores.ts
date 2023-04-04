import { writable, type Writable } from "svelte/store";
import type { Message } from "./interfaces";

export const socket: Writable<WebSocket | null> = writable(null);
export const messages: Writable<Message[]> = writable([]);