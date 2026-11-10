// Lets the screens say the right thing on each computer.
export const onMac = navigator.userAgent.includes("Macintosh");
export const computerName = onMac ? "Mac" : "PC";
