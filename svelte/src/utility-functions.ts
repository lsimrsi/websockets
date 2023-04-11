import { TOAST_DURATION } from './constants';
import type { ServerMessage, Toast, ToastItem } from './interfaces';
import { socket, toasts } from './stores';
import { get } from 'svelte/store';
import { v4 as uuidv4 } from "uuid";

export function sendMessage(msg: ServerMessage) {
  let $socket = get(socket);
  if (!socket) {
    console.log('Socket was null');
    return;
  }
  console.log('sending msg', msg);
  $socket.send(JSON.stringify(msg));
}
