import java.io.File

def main(): Unit = {
  val cbtcDARDir = new File("./dars/cbtc")
  val cbtcDARs = if (cbtcDARDir.exists && cbtcDARDir.isDirectory) {
    cbtcDARDir.listFiles
      .filter(_.getName.endsWith(".dar"))
      .map(_.getPath)
  } else {
    throw new RuntimeException("./dars/cbtc directory not found")
  }

  println("Uploading CBTC DARs...")

  cbtcDARs.foreach { path =>
    println(s"Uploading/Checking CBTC DAR: $path")
    participant.dars.upload(path)
  }

  println("All CBTC DARs uploaded.")

  val dependencyDARDir = new File("./dars/dependencies")
  val dependencyDARs = if (dependencyDARDir.exists && dependencyDARDir.isDirectory) {
    dependencyDARDir.listFiles
      .filter(_.getName.endsWith(".dar"))
      .map(_.getPath)
  } else {
    throw new RuntimeException("./dars/dependencies directory not found")
  }

  println("Uploading dependency DARs...")

  dependencyDARs.foreach { path =>
    println(s"Uploading/Checking dependency DAR: $path")
    participant.dars.upload(path)
  }

  println("All dependency DARs uploaded.")

}

main()
