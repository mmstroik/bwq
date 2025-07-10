use crate::ExitStatus;

pub fn run_examples() -> Result<ExitStatus, anyhow::Error> {
    show_examples();
    Ok(ExitStatus::Success)
}

fn show_examples() {
    println!("Brandwatch Query Examples:");
    println!();

    println!("Basic Boolean Operators:");
    println!("  apple AND juice");
    println!("  apple OR orange");
    println!("  apple NOT bitter");
    println!("  (apple OR orange) AND juice");
    println!();

    println!("Quoted Phrases:");
    println!("  \"apple juice\"");
    println!("  \"organic fruit\" AND healthy");
    println!();

    println!("Proximity Operators:");
    println!("  \"apple juice\"~5");
    println!("  apple NEAR/3 juice");
    println!("  apple NEAR/2f juice");
    println!();

    println!("Wildcards and Replacement:");
    println!("  appl*");
    println!("  customi?e");
    println!();

    println!("Field Operators:");
    println!("  title:\"apple juice\"");
    println!("  site:twitter.com");
    println!("  author:brandwatch");
    println!("  language:en");
    println!("  rating:[3 TO 5]");
    println!();

    println!("Location Operators:");
    println!("  country:usa");
    println!("  region:usa.ca");
    println!("  city:\"usa.ca.san francisco\"");
    println!();

    println!("Advanced Operators:");
    println!("  authorFollowers:[1000 TO 50000]");
    println!("  engagementType:RETWEET");
    println!("  authorGender:F");
    println!("  {{BrandWatch}}  (case-sensitive)");
    println!();

    println!("Comments:");
    println!("  apple <<<This is a comment>>> AND juice");
    println!();

    println!("Special Characters:");
    println!("  #MondayMotivation");
    println!("  @brandwatch");
    println!();
}
