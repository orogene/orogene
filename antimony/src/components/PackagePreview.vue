<template>
  <section class="preview">
    <header>
      <h1>node-wumbo</h1>
      <p class="author">Patrick Star</p>
    </header>
    <div class="actions">
      <button class="install">Install</button>
      <button class="view">View Repository</button>
    </div>
    <div class="readme">
      <!-- TODO replace this with markdown content -->
      <p>Wumbo! Everybody loves wumbo.</p>
    </div>
  </section>
</template>

<style scoped>
.preview {
  width: 100%;
  background-color: #2d2d2d;
  border: 2px solid;
  border-color: #828282;
  border-radius: 2px;

  padding: 1.5rem;

  text-align: left;
}

h1 {
  font-size: 1.25rem;
  font-weight: normal;
  color: #f2f2f2;
  margin: 0;
  margin-bottom: 0.25rem;
}

.author {
  font-size: 1.25rem;
  margin: 0;
  margin-bottom: 0.75rem;
}

button {
  margin-right: 0.5rem;
  border: none;
  border-radius: 1rem;
  padding: 0.125rem 1.5rem;
  color: #434343;
  font-weight: bold;
  text-transform: uppercase;
}

button.install {
  background-color: #c9e774;
}

button.view {
  background-color: #b08aed;
}

.readme p {
  font-size: 0.875rem;
}
</style>
