import type { ServerMessage } from './interfaces';
import { socket } from './stores';
import { get } from 'svelte/store';

export function sendMessage(msg: ServerMessage) {
  let $socket = get(socket);
  if (!socket) {
    console.log('Socket was null');
    return;
  }
  console.log('sending msg', msg);
  $socket.send(JSON.stringify(msg));
}
