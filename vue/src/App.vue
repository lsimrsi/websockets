<script setup lang="ts">
import { ref, onMounted, onUpdated, type Ref } from 'vue'
import { useHasRegisteredNameStore, useMessagesStore, useNameStore, useSocketStore, useToastsStore } from '@/stores'
import Login from './components/Login.vue'
import Input from './components/Input.vue'
import ChatMessage from './components/ChatMessage.vue'
import { ToastType } from './interfaces';
import { v4 as uuidv4 } from "uuid";

let message = ref("");
let ul: Ref<HTMLUListElement | null> = ref(null);

const messages = useMessagesStore();
const socket = useSocketStore();
const toasts = useToastsStore();
const name = useNameStore();
const hasRegisteredName = useHasRegisteredNameStore();

onUpdated(() => {
  ul.value?.scrollTo(0, ul.value.scrollHeight);
})

onMounted(() => {
    if (socket.socket != null) return;

    let newSocket = new WebSocket("ws://127.0.0.1:8080/ws");

    newSocket.onopen = function (this: WebSocket, ev: Event) {
      console.log("Connected");
    };

    newSocket.onclose = function () {
      console.log("Disconnected");
      socket.socket = null;
    };

    newSocket.onmessage = function (res: MessageEvent<any>) {
      console.log("res.data", res.data);

      let serverMsg = JSON.parse(res.data);

      switch (serverMsg.msg_type) {
        case "AllMessages":
          messages.messages = serverMsg.data;
          break;
        case "NewMessage":
          messages.messages = [...messages.messages, serverMsg.data];
          break;
        case "NameTaken":
          let item = {
            type: ToastType.Client,
            message: "Name has already been taken.",
            uuid: uuidv4(),
          };
          toasts.add(item);
          hasRegisteredName.hasRegisteredName = false;
          break;
        case "NameRegistered":
          console.log("Name registered");
          hasRegisteredName.hasRegisteredName = true;
          break;
        case "Joined":
          messages.messages = [
            ...messages.messages,
            { name: serverMsg.data, message: "Hello, world!" },
          ];
          break;
      }

      console.log("messages", messages.messages);
    };

    socket.socket = newSocket;
  });

  function onSubmit(e: any) {
    e.preventDefault();
    if (socket.socket == null) {
      console.log("Socket is null");
      return;
    }

    if (message.value.trim() === "") {
      console.log("Message was empty.");
      return;
    }

    socket.sendMessage({ msg_type: "Chat", data: { name: name.name, message: message.value } });
    message.value = "";
  }

</script>

<template>
<main class="flex h-screen overflow-hidden bg-zinc-50">
  <!-- <Toasts /> -->
    <Login v-if="!hasRegisteredName.hasRegisteredName"/>
    <div v-else class="grid grid-cols-1 grid-rows-2 m-auto h-4/5 w-4/5 gap-2">
      <ul
        class="overflow-y-scroll p-4 border rounded-lg bg-white"
        ref="ul"
      >
          <li v-for="msg in messages.messages"><ChatMessage :chatMsg="msg" /></li>
      </ul>

      <form class="flex flex-col" @submit="onSubmit">
        <Input
          classes="mb-2"
          label="Enter message"
          id="chat-message"
          v-model="message"
        />
        <input type="submit" classes="w-fit" id="send-message" value="Send" />
      </form>
    </div>
</main>
</template>

<style scoped>
</style>
