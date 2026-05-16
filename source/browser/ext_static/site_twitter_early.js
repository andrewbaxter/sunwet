// Early content script (document_start, ISOLATED world) that injects a page-level
// fetch interceptor to capture Twitter video URLs from API responses.
const s = document.createElement('script');
s.textContent = `(function() {
  const store = document.createElement('div');
  store.id = 'sunwet-video-store';
  store.style.display = 'none';
  document.documentElement.appendChild(store);

  const origFetch = window.fetch;
  window.fetch = async function(...args) {
    const response = await origFetch.apply(this, args);
    try {
      const url = (typeof args[0] === 'string') ? args[0] : args[0]?.url;
      if (url && (url.includes('/TweetDetail') || url.includes('/TweetResultByRestId') || url.includes('/UserTweets') || url.includes('/HomeTimeline') || url.includes('/HomeLatestTimeline') || url.includes('/ListLatestTweetsTimeline'))) {
        const clone = response.clone();
        clone.json().then(json => {
          const findTweetVideos = (obj, depth) => {
            if (!obj || typeof obj !== 'object' || depth > 20) return;
            if (obj.rest_id && obj.legacy && obj.legacy.extended_entities && obj.legacy.extended_entities.media) {
              const tweetId = obj.rest_id;
              for (const em of obj.legacy.extended_entities.media) {
                if (em.video_info && em.video_info.variants) {
                  const mp4s = em.video_info.variants
                    .filter(v => v.content_type === 'video/mp4' && v.bitrate)
                    .sort((a, b) => b.bitrate - a.bitrate);
                  if (mp4s.length > 0) {
                    store.setAttribute('data-tweet-' + tweetId, mp4s[0].url);
                  }
                }
              }
            }
            if (Array.isArray(obj)) {
              for (const item of obj) findTweetVideos(item, depth + 1);
            } else {
              for (const val of Object.values(obj)) findTweetVideos(val, depth + 1);
            }
          };
          findTweetVideos(json, 0);
        }).catch(() => {});
      }
    } catch(e) {}
    return response;
  };
})();`;
document.documentElement.prepend(s);
