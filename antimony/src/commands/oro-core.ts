import { promisified } from 'tauri/api/tauri';
import { ref } from 'vue';

interface PingResponse {
  body: {
    registry: string;
    time: number;
    details: object;
  };
}

/* usage in vue component example:
  in script:
  
  ```js
  import { usePing } from '@/commands/oro-core'
  export default {
    setup () {
      const { loading, ping, response } = usePing();
  
      return { loading, ping, response }
    }
  }
  ```

  in template
  ```html
  <button @click="ping" :disabled="loading">ping!</button>
  <div v-if="loading">Currently loading...</div>
  <pre v-else-if="response">{{ response }}</pre>
 */

export function usePing() {
  const loading = ref(false);
  const response = ref<PingResponse | null>(null);

  async function ping() {
    try {
      loading.value = true;

      response.value = await promisified<PingResponse>({
        cmd: 'ping',
        args: null
      });
    } catch (e) {
      console.error(e);
    } finally {
      loading.value = false;
    }
  }

  return {
    loading,
    response,
    ping
  };
}
