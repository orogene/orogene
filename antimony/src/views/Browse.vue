<template>
  <div class="browse">
    <PackageSearch />
    <PackagePreview />
  </div>
</template>

<script>
import PackageSearch from '@/components/PackageSearch.vue';
import PackagePreview from '@/components/PackagePreview.vue';

export default {
  name: 'Home',
  components: {
    PackageSearch,
    PackagePreview
  }
};
</script>

<style scoped>
.browse {
  margin: 1rem;
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-gap: 1rem;
}
</style>
