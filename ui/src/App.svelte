<script lang="ts">
  import { messages, socket } from "./stores";
  import { onMount } from "svelte";
  import type { Message, ServerMessage } from "./interfaces";

  let name = "";
  let message = "";

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
    $socket.send(JSON.stringify({ name, message }));
  }
</script>

<main>
  <ul>
    {#each $messages as msg}
      <li>{msg.name}: {msg.message}</li>
    {/each}
  </ul>

  <form class="flex flex-col bg-yellow-400" on:submit={onSubmit}>
    <label for="name">Enter name:</label>
    <input id="name" bind:value={name} />

    <label for="chat-message">Enter message:</label>
    <textarea id="chat-message" bind:value={message} />
    <input type="submit" value="Send" />
  </form>
</main>

<style>
</style>
