export interface ServerMessage {
  msg_type: string;
  data: any;
}

export interface ChatMessage {
  name: string;
  message: string;
}