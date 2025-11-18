def main(): Unit = {
  val dars = participant.dars.list()

  dars.foreach { dar =>
    if (dar.name.toLowerCase.contains("cbtc")) {
      println(
        s"""
        |DAR:
        |  Name         : ${dar.name}
        |  Version      : ${dar.version}
        |  MainPackageId: ${dar.mainPackageId}
        |  Description  : ${dar.description}
        """.stripMargin)
    }
  }
}

main()