// This is for delaying any requests until typing as stopped (for 500ms).
// When the timer expires, any existing request is canceled and a new
// request is sent.
const REQUEST_DELAY = 500;
let typingTimer = null;
// This is the pending request, if any. 
let xhr = null;

// This is a trick for installing listener after the page is ready.
document.addEventListener("DOMContentLoaded", function(event) { 
  let inputBox = document.getElementById('term');
  inputBox.addEventListener('keyup', () => {
    clearTimeout(typingTimer);
    typingTimer = setTimeout(doneTyping, REQUEST_DELAY);
  })
});

function doneTyping() {
  let text = document.getElementById("term").value;
  if (text.length > 0) {
    document.getElementById("lookupoutput").innerHTML = "<em>searching ...</em>";
  } else {
    document.getElementById("lookupoutput").innerHTML = "";
    if (xhr != null) {
      console.log('Canceled pending request because of empty input.');
      xhr.abort();
      xhr = null;
    }
    return;
  }
  if (xhr != null) {
    console.log('Canceled pending request for new query.');
    xhr.abort();
  }
  xhr = new XMLHttpRequest();
  xhr.onload = () => {
    if (xhr.status == 200) {
      document.getElementById("lookupoutput").innerHTML = xhr.response;
    } else {
      console.error('Error!');
    }
    xhr = null;
  };
  
  xhr.open('GET', 'http://127.0.0.1:8000/api/lookup?term=' + text);
  xhr.send();
}
