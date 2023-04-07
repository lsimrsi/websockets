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

/** Add a new toast to the queue. */
function addNewToast(toast: Toast) {
  let $toasts = get(toasts);
  let uuid = uuidv4();
  let item = {
    type: toast.type,
    message: toast.message,
    uuid,
    show: true,
  };

  $toasts = [...$toasts, item];

  setTimeout(() => {
    let $toastsLater = get(toasts);
    let filteredToasts = $toastsLater.filter(
      (ti: ToastItem) => ti.uuid !== item.uuid
    );
    $toastsLater = [...filteredToasts];
  }, TOAST_DURATION);
}