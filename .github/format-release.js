const analysis = process.argv[2];

console.log(
  analysis
    .split(",")
    .map((line) => {
      const [file, url] = line.split("=", 2);
      return `\`${file}\`: [VirusTotal analysis](${url})`;
    })
    .join("\n")
);
