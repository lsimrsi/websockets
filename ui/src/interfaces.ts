export interface ServerMessage {
  msg_type: string;
  data: any;
}

export interface ChatMessage {
  name: string;
  message: string;
}

/** Temporary message that is displayed to the user. */
export interface Toast {
  type: ToastType;
  message: string;
}

/** Manages state for the toast queue. */
export interface ToastItem extends Toast {
  uuid: string;
}

export enum ToastType {
  Network = "Network",
  Info = "Info",
  Success = "Success",
  Redirect = "Redirect",
  Client = "Client Error",
  Server = "Server Error",
}