<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed } from 'vue'
import Navbar from './components/Navbar.vue'

const route = useRoute()
const showNavbar = computed(() => route.name !== 'Home')
</script>

<template>
  <div id="app">
    <Navbar v-if="showNavbar" />
    <div :class="showNavbar ? 'container py-4' : ''">
      <router-view />
    </div>
  </div>
</template>

<style>
#app {
  min-height: 100vh;
}
</style>
