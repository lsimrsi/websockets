<script lang="ts">
  import { hasRegisteredName, messages, socket, name } from "./stores";
  import { afterUpdate, onMount } from "svelte";
  import Login from "./lib/login.svelte";
  import { sendMessage } from "./utility-functions";
  import Button from "./lib/button.svelte";
  import Input from "./lib/input.svelte";
  import ChatMessage from "./lib/chat-message.svelte";

  let message = "";
  let chatWindow: HTMLUListElement | null = null;

  afterUpdate(() => {
    if (chatWindow) {
      chatWindow.scrollTo(0, chatWindow.scrollHeight);
    }
  });

  onMount(async () => {
    if ($socket != null) return;

    let newSocket = new WebSocket("ws://127.0.0.1:8080/ws");

    newSocket.onopen = function (this: WebSocket, ev: Event) {
      console.log("Connected");
    };

    newSocket.onclose = function () {
      console.log("Disconnected");
      $socket = null;
    };

    newSocket.onmessage = function (res: MessageEvent<any>) {
      console.log("res.data", res.data);

      let serverMsg = JSON.parse(res.data);

      switch (serverMsg.msg_type) {
        case "AllMessages":
          $messages = serverMsg.data;
          break;
        case "NewMessage":
          $messages = [...$messages, serverMsg.data];
          break;
        case "NameTaken":
          console.log("Name taken");
          $hasRegisteredName = false;
          break;
        case "NameRegistered":
          console.log("Name registered");
          $hasRegisteredName = true;
          break;
      }

      console.log("messages", $messages);
    };

    $socket = newSocket;
  });

  function onSubmit(e) {
    e.preventDefault();
    if ($socket == null) {
      console.log("Socket is null");
      return;
    }
    sendMessage({ msg_type: "Chat", data: { name: $name, message } });
    message = "";
  }
</script>

<main class="flex h-screen overflow-hidden bg-zinc-50">
  {#if !$hasRegisteredName}
    <Login />
  {:else}
    <div class="grid grid-cols-1 grid-rows-2 m-auto h-4/5 w-4/5 gap-2">
      <ul
        class="overflow-y-scroll border rounded-lg p-4"
        bind:this={chatWindow}
      >
        {#each $messages as msg}
          <li><ChatMessage chatMsg={msg} /></li>
        {/each}
      </ul>

      <form class="flex flex-col" on:submit={onSubmit}>
        <Input
          classes="mb-2"
          label="Enter message"
          id="chat-message"
          bind:value={message}
        />
        <Button classes="w-fit" id="send-message" value="Send" />
      </form>
    </div>
  {/if}
</main>

<style>
</style>
