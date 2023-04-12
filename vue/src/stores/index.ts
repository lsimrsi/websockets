import { ref, type Ref } from 'vue'
import { defineStore } from 'pinia'
import type { ChatMessage } from '@/interfaces';

export const useSocketStore = defineStore('socket', () => {
  const socket: Ref<WebSocket | null> = ref(null);
  function set(newSocket: WebSocket | null) {
    socket.value = newSocket;
  }

  return { socket, set }
})

export const useMessagesStore = defineStore('messages', () => {
  const messages: Ref<ChatMessage[]> = ref([])
  function add(chatMsg: ChatMessage) {
    messages.value = [...messages.value, chatMsg];
  }

  return { messages, add }
})

export const useNameStore = defineStore('name', () => {
  const name: Ref<string> = ref("");
  function set(newName: string) {
    name.value = newName;
  }

  return { name, set }
})

export const useHasRegisteredNameStore = defineStore('hasRegisteredName', () => {
  const hasRegisteredName: Ref<boolean> = ref(false);
  function set(didRegister: boolean) {
    hasRegisteredName.value = didRegister;
  }

  return { hasRegisteredName, set }
})