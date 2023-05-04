
/* Code modified from the blender website
 * https://www.blender.org/wp-content/themes/bthree/assets/js/get_os.js?x82196
 */

let options = {
  windows: "pc-windows",
  windows64: "64-pc-windows",
  windowsArm: "arm-pc-windows",

  mac: "apple-darwin",
  macPc: "apple-ppc",
  mac32: "apple-32",
  macSilicon: "apple-silicon",

  linux: "unknown-linux",
  linuxUbuntu: "linux-ubuntu",
  linuxDebian: "linux-debian",
  linuxMandriva: "linux-mandriva",
  linuxRedhat: "linux-redhat",
  linuxFedora: "linux-fedora",
  linuxSuse: "linux-suse",
  linuxGentoo: "linux-gentoo",

  ios: "ios",
  android: "linux-android",

  freebsd: "freebsd",
};

function isAppleSilicon() {
  try {
    var glcontext = document.createElement("canvas").getContext("webgl");
    var debugrenderer = glcontext
      ? glcontext.getExtension("WEBGL_debug_renderer_info")
      : null;
    var renderername =
      (debugrenderer &&
        glcontext.getParameter(debugrenderer.UNMASKED_RENDERER_WEBGL)) ||
      "";
    if (renderername.match(/Apple M/) || renderername.match(/Apple GPU/)) {
      return true;
    }

    return false;
  } catch (e) {}
}

function getOS() {
  var OS = options.windows.default;
  var userAgent = navigator.userAgent;
  var platform = navigator.platform;

  if (navigator.appVersion.includes("Win")) {
    if (
      !userAgent.includes("Windows NT 5.0") &&
      !userAgent.includes("Windows NT 5.1") &&
      (userAgent.indexOf("Win64") > -1 ||
        platform == "Win64" ||
        userAgent.indexOf("x86_64") > -1 ||
        userAgent.indexOf("x86_64") > -1 ||
        userAgent.indexOf("amd64") > -1 ||
        userAgent.indexOf("AMD64") > -1 ||
        userAgent.indexOf("WOW64") > -1)
    ) {
      OS = options.windows64;
    } else {
      if (
        window.external &&
        window.external.getHostEnvironmentValue &&
        window.external
          .getHostEnvironmentValue("os-architecture")
          .includes("ARM64")
      ) {
        OS = options.windowsArm;
      } else {
        try {
          var canvas = document.createElement("canvas");
          var gl = canvas.getContext("webgl");

          var debugInfo = gl.getExtension("WEBGL_debug_renderer_info");
          var renderer = gl.getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
          if (renderer.includes("Qualcomm")) OS = options.windowsArm;
        } catch (e) {}
      }
    }
  }

  //MacOS, MacOS X, macOS
  if (navigator.appVersion.includes("Mac")) {
    if (platform.includes("MacPPC") || platform.includes("PowerPC")) {
      OS = options.macPpc;
    } else if (
      navigator.userAgent.includes("OS X 10.5") ||
      navigator.userAgent.includes("OS X 10.6")
    ) {
      OS = options.mac32;
    } else {
      OS = options.mac;

      const isSilicon = isAppleSilicon();
      if (isSilicon) {
        OS = options.macSilicon;
      }
    }
  }

  // linux
  if (platform.includes("Linux")) {
    if (navigator.userAgent.toLocaleLowerCase().includes("ubuntu"))
      OS = options.linux_ubuntu;
    else if (userAgent.includes("Debian")) OS = options.linuxDebian;
    else if (userAgent.includes("Android")) OS = options.android;
    else if (userAgent.includes("Mandriva")) OS = options.linuxMandriva;
    else if (userAgent.includes("Red Hat")) OS = options.linuxRedhat;
    else if (userAgent.includes("Fedora")) OS = options.linuxFedora;
    else if (userAgent.includes("SUSE")) OS = options.linuxSuse;
    else if (userAgent.includes("Gentoo")) OS = options.linuxGentoo;
    else OS = options.linux;
  }

  if (
    userAgent.includes("iPad") ||
    userAgent.includes("iPhone") ||
    userAgent.includes("iPod")
  ) {
    OS = options.ios;
  }
  if (platform.toLocaleLowerCase().includes("freebsd")) {
    OS = options.freebsd;
  }

  return OS;
}

let os = getOS();
window.os = os;

let hit = Array.from(document.querySelectorAll(".target[data-targets]")).find(
  (a) => a.attributes["data-targets"].value.includes(os)
);
let backupButton = document.querySelector(".backup-download");
if (hit) {
  hit.classList.remove("hidden");
} else {
  if (window.os === options.macSilicon) {
    const macDownloadButtons = Array.from(
      document.querySelectorAll(".target[data-targets]")
    ).find((a) => a.attributes["data-targets"].value.includes(options.mac));
    if (macDownloadButtons) {
      macDownloadButtons.classList.remove("hidden");
    }
  } else if (backupButton) {
    backupButton.classList.remove("hidden");
  }
}

let copyButtons = Array.from(document.querySelectorAll("[data-copy]"));
if (copyButtons.length) {
  copyButtons.forEach(function (element) {
    element.addEventListener("click", () => {
      navigator.clipboard.writeText(element.attributes["data-copy"].value);
    });
  });
}

// Toggle for pre releases
const checkbox = document.getElementById("show-prereleases");

if (checkbox) {
  checkbox.addEventListener("click", () => {
    const all = document.getElementsByClassName("pre-release");

    if (all) {
      for (var item of all) {
        item.classList.toggle("hidden");
      }
    }
  });
}
