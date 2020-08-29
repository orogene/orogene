<template>
  <section>
    <input type="search" />
    <ul>
      <li v-for="pkg in packages" :key="pkg.name">
        <div>
          <div class="heading">
            <h2>{{ pkg.name }}</h2>
            <div class="stats">
              <span>{{ pkg.downloads }}</span>
              <span>{{ pkg.version }}</span>
            </div>
          </div>
          <p>{{ pkg.excerpt }}</p>
        </div>
      </li>
    </ul>
  </section>
</template>

<script>
const mockPackage = {
  name: 'node-wumbo',
  excerpt:
    'You should use W for wumbo. You know? Wumbo? I wumbo, you wumbo, he, she, they wumbo?',
  downloads: '47k', // TODO this should be a formatted number
  version: '69.420.0'
};

const packages = [mockPackage, mockPackage, mockPackage, mockPackage];

export default {
  data() {
    return {
      packages
    };
  }
};
</script>

<style scoped>
section {
  width: 100%;
  height: 28rem; /* FIXME should fill container */
  background-color: #2d2d2d;
  border-top-left-radius: 0.5rem;
  border-top-right-radius: 0.5rem;
}

input {
  margin: 0.5rem;
  /* FIXME why isn't this just 100%?? */
  width: calc(100% - 1rem);
  border-radius: 1.5rem;
  height: 1.5rem;
  border: none;
  background-color: #434343;
  padding-left: 2rem;
  color: #f2f2f2;
}

ul {
  padding: 0;
  margin: 0;
}

li {
  margin: 0.25rem;
  padding: 0.5rem;
  list-style-type: none;
  text-align: left;
}

li:nth-child(odd) {
  background-color: #434343;
}

.heading {
  display: flex;
  margin-bottom: 0.75rem;
}

.stats {
  margin-left: auto;
}

.stats span {
  margin-left: 1rem;
}

h2 {
  margin: 0;
  font-size: 1rem;
}

p {
  margin: 0;
  font-size: 0.875rem;
}
</style>
