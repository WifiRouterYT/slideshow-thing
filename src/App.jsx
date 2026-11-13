import { createSignal, onMount } from "solid-js";
import { createStore } from "solid-js/store";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import "./App.css";

import { MdFillError } from 'solid-icons/md'

const sleep = ms => new Promise(r => setTimeout(r, ms));

function App() {
  const [isDev, setDevState] = createSignal(false);

  const [loading, setLoading] = createSignal(true);
  const [errorState, setErrorState] = createSignal(false);

  const [errorTitle, setErrorTitle] = createSignal(null);
  const [errorBody, setErrorBody] = createSignal(null);
  const [errorSolution, setErrorSolution] = createSignal(null);
  const [errorBoxWidth, setErrorBoxWidth] = createSignal(245);
  const [errorBoxHeight, setErrorBoxHeight] = createSignal(65);

  const [imageList, setImageList] = createStore([]);
  const [currentImage, setCurrentImage] = createSignal(null);

  async function scrollImages() {
    let currentIndex = 0;
    while(true) {
      await validate();
      if(!errorState()) {
        if(currentIndex > imageList.length) currentIndex = 0;
        console.log("current image: " + imageList[currentIndex]);
        let displayPath = imageList[currentIndex];
        try {
          displayPath = await invoke("get_display_path", { path: displayPath });
        } catch(e) {
          console.error(e);
        }
        setCurrentImage(displayPath);
        currentIndex++;
        if(currentIndex === imageList.length) currentIndex = 0;
      }
      await sleep(5000);
    }
  }

  async function validate() {
    console.log("running validation");
    const validate = await invoke("validate_path");
    setLoading(false);
    if(validate != "OK") {
      setErrorBoxHeight(150);
      switch(validate) {
        case "fail::invalid_usb":
          setErrorTitle("No image folder found.");
          setErrorBody("Could not find an image folder on the USB device.");
          setErrorSolution("Create a folder called 'img' on the device and put images in there.");
          setErrorState(true);
          setErrorBoxWidth(650);
          break;
        case "fail::not_setup":
          setErrorTitle("Setup incomplete.");
          setErrorBody("This system is not setup yet.");
          setErrorSolution("Run './kioskapp setup' and reboot.");
          setErrorState(true);
          setErrorBoxWidth(500);

          break;
        case "fail::no_img":
          setErrorTitle("No images found.");
          setErrorBody("The mounted USB device does not have any valid images.");
          setErrorSolution("Add the following supported image types: png, jpg, jpeg");
          setErrorState(true);
          setErrorBoxWidth(600);

          break;
        case "fail::read_fail":
          setErrorTitle("Failed to read USB device.");
          setErrorBody("An error occurred while opening the storage device.");
          setErrorSolution("Format the drive as FAT32 and try again.");
          setErrorState(true);
          setErrorBoxWidth(600);

          break;
        case "fail::no_device":
          setErrorTitle("No device found.");
          setErrorBody("Couldn't find a USB device with images to play!");
          setErrorSolution("Insert a FAT32 formatted USB stick with images.");
          setErrorState(true);
          setErrorBoxWidth(550);
      }
      return false;
    }
    setErrorState(false);

    const imageList = await invoke("get_images");
    setImageList(imageList);

    const listSize = imageList.length;
    if(listSize === 1) {
      invoke("get_display_path", { path: imageList[0] })
        .then(p => setCurrentImage(p))
        .catch(() => setCurrentImage(imageList[0]));
      return;
    }
  }

  async function loop() {
    await validate();
    await sleep(5000);
  }

  onMount(async () => {
    // check if on dev env
    const dev = await invoke('check_dev');
    setDevState(dev);

    await validate();

    if(!errorState()) {
      // warm the thumbnail cache in the background so scrolling never
      // waits on cold decodes of 18MP photos
      invoke("start_thumb_pregen").catch(e => console.error(e));
    }

    Promise.all([scrollImages(), loop()])
  })

  function heightCSS() {
    return `width: ${errorBoxWidth()}px; height: ${errorBoxHeight()}px;`;
  }

  return (
    <main class="container">
      <Show when={loading() || errorState()}>
        <div class={"box" + (errorState() ? " error" : "")} style={heightCSS()}>
          <Show when={loading()}>
            <div class="loader"></div>
            <p>Starting up...</p>
          </Show>

          <Show when={errorState()}>
            <div class="icon">
              <MdFillError />
            </div>
            <div class="text">
              <p class="title">{errorTitle}</p>
              <p class="body">{errorBody}</p>
              <br></br>
              <p class="solution"><b>Solution: </b>{errorSolution}</p>
            </div>
          </Show>
        </div>
      </Show>
      <Show when={!(loading() || errorState())}>
        <div class="slideshow">
          <img src={convertFileSrc(currentImage())}></img>
        </div>
      </Show>
    </main>
  );
}

export default App;
