// QUIZ APP
//
use rand::seq::SliceRandom;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Instant;

const QUESTIONS_FILE: &str = "questions.txt";
const SCORES_FILE: &str = "scores.txt";
const TOP_SCORES: usize = 15; // rows shown in the high-score table
const QUESTION_TIME_LIMIT_SECS: u64 = 20; // time allowed per question

// NOTE ON '|' AND '~'
// Both data files use '|' to separate fields, so a '|' typed by the
// player would split a line into extra fields. read_input() therefore
// stores every '|' as '~'. Names and categories turn '~' back into '|'
// only when they are displayed.

// =====================================================
// ENUM WITH ASSOCIATED DATA
//
// Each variant can hold different data inside it.
// MultiChoice holds a Vec of options and a char answer.
// TrueFalse holds a bool answer.
// =====================================================
enum QuestionKind {
    MultiChoice { options: Vec<String>, answer: char },
    TrueFalse { answer: bool },
}

// =====================================================
// QUESTION STRUCT
// =====================================================
struct Question {
    category: String,
    difficulty: String,
    text: String,
    kind: QuestionKind,
}

impl Question {
    // Parse one line from the file into a Question.
    //
    // MC format: MC|Category|Difficulty|Question|A|B|C|D|CorrectLetter
    // TF format: TF|Category|Difficulty|Question|True/False
    fn parse(line: &str) -> Option<Question> {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            return None;
        }

        let parts: Vec<&str> = line.split('|').collect();

        match parts.get(0)?.trim() {
            "MC" => {
                if parts.len() < 9 {
                    return None;
                }
                let answer_char = parts[8].trim().to_uppercase().chars().next()?;

                if !"ABCD".contains(answer_char) {
                    return None;
                }

                Some(Question {
                    category: parts[1].trim().to_string(),
                    difficulty: capitalize(parts[2].trim()), // normalize casing on load
                    text: parts[3].trim().to_string(),
                    kind: QuestionKind::MultiChoice {
                        options: vec![
                            format!("A) {}", parts[4].trim()),
                            format!("B) {}", parts[5].trim()),
                            format!("C) {}", parts[6].trim()),
                            format!("D) {}", parts[7].trim()),
                        ],
                        answer: answer_char,
                    },
                })
            }

            "TF" => {
                if parts.len() < 5 {
                    return None;
                }

                let answer = match parts[4].trim().to_lowercase().as_str() {
                    "true" => true,
                    "false" => false,
                    _ => return None,
                };

                Some(Question {
                    category: parts[1].trim().to_string(),
                    difficulty: capitalize(parts[2].trim()), // normalize casing on load
                    text: parts[3].trim().to_string(),
                    kind: QuestionKind::TrueFalse { answer },
                })
            }

            _ => None,
        }
    }

    // Points awarded — harder questions are worth more
    fn points(&self) -> u32 {
        match self.difficulty.to_lowercase().as_str() {
            "easy" => 1,
            "medium" => 2,
            "hard" => 3,
            _ => 1,
        }
    }
}

// =====================================================
// WRONG ANSWER RECORD
// =====================================================
struct WrongAnswer {
    question_text: String,
    player_answer: String,
    correct_answer: String,
}

// =====================================================
// SCORE STRUCT
// =====================================================
struct Score {
    name: String,
    correct: u32,
    total: u32,
    points: u32,
    max_points: u32,
    time_secs: u64,
    category: String,
    best_streak: u32,
}

impl Score {
    fn percentage(&self) -> u32 {
        if self.total == 0 {
            0
        } else {
            (self.correct * 100) / self.total
        }
    }

    fn to_save_line(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            self.name,
            self.correct,
            self.total,
            self.points,
            self.max_points,
            self.time_secs,
            self.category,
            self.best_streak
        )
    }

    fn from_save_line(line: &str) -> Option<Score> {
        let p: Vec<&str> = line.split('|').collect();
        if p.len() < 7 {
            return None;
        }
        Some(Score {
            name: p[0].to_string(),
            correct: p[1].parse().ok()?,
            total: p[2].parse().ok()?,
            points: p[3].parse().ok()?,
            max_points: p[4].parse().ok()?,
            time_secs: p[5].parse().ok()?,
            category: p[6].to_string(),
            // Older score lines have no streak field, so they count as 0.
            best_streak: if p.len() > 7 { p[7].parse().ok()? } else { 0 },
        })
    }

    fn grade(&self) -> &str {
        match self.percentage() {
            90..=100 => "S",
            80..=89 => "A",
            70..=79 => "B",
            60..=69 => "C",
            50..=59 => "D",
            _ => "F",
        }
    }
}

// =====================================================
// HELPER — Capitalize first letter, lowercase rest.
//
// "easy"   -> "Easy"
// "HARD"   -> "Hard"
// "mEdIuM" -> "Medium"
//
// Used to normalize difficulty names so they always
// match the quiz filter, whatever casing was typed.
// =====================================================
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
    }
}

// =====================================================
// FILE — create a starter questions file
// =====================================================
fn create_sample_questions() {
    let content = r#" // QUIZ QUESTIONS FILE
// MC format: MC|Category|Difficulty|Question|OptionA|OptionB|OptionC|OptionD|CorrectLetter
// TF format: TF|Category|Difficulty|Question|True/False
//
// Difficulties: Easy, Medium, Hard
// Add your own questions below!

// --- GEOGRAPHY ---
MC|Geography|Easy|What is the capital of France?|Paris|London|Berlin|Madrid|A
MC|Geography|Easy|Which is the largest ocean on Earth?|Pacific|Atlantic|Indian|Arctic|A
MC|Geography|Medium|What is the capital of Australia?|Sydney|Melbourne|Canberra|Brisbane|C
MC|Geography|Medium|Which strait has passed through Istanbul?|Gibraltar|Malacca|Bosphorus|Bab-el-mandeb|C
MC|Geography|Medium|Which country has the most natural lakes?|USA|Canada|Russia|Brazil|B
MC|Geography|Hard|What is the smallest country in the world by area?|Monaco|San Marino|Vatican City|Liechtenstein|C
TF|Geography|Easy|The Amazon River is the longest river in the world.|False
TF|Geography|Medium|Australia is both a country and a continent.|True
TF|Geography|Hard|The Dead Sea is the saltiest body of water on Earth.|False

// --- SCIENCE ---
MC|Science|Easy|What is the chemical symbol for water?|HO|H2O|H3O|OH|B
MC|Science|Easy|How many planets are in our solar system?|7|8|9|10|B
MC|Science|Medium|What is the chemical symbol for gold?|Go|Gd|Au|Ag|C
MC|Science|Medium|What gas do plants absorb from the atmosphere?|Oxygen|Nitrogen|Carbon Dioxide|Hydrogen|C
MC|Science|Hard|What is the speed of light in a vacuum approximately?|300 km/s|3000 km/s|300000 km/s|3000000 km/s|C
TF|Science|Easy|The sun is a star.|True
TF|Science|Medium|Sound travels faster than light.|False
TF|Science|Hard|DNA stands for Deoxyribonucleic Acid.|True

// --- TECHNOLOGY ---
MC|Technology|Easy|What does CPU stand for?|Central Processing Unit|Computer Power Unit|Central Program Utility|Core Processing Unit|A
MC|Technology|Easy|What programming language is known as the language of the web (frontend)?|Python|Java|JavaScript|Ruby|C
MC|Technology|Medium|What does RAM stand for?|Random Access Memory|Read Access Memory|Rapid Application Memory|Random Allocation Memory|A
MC|Technology|Medium|Which company helped to develop the Rust programming language?|Google|Mozilla|Microsoft|Apple|B
MC|Technology|Hard|What does HTTPS stand for?|HyperText Transfer Protocol Secure|High Transfer Text Protocol System|HyperText Transmission Protocol Standard|High Text Transfer Protocol Secure|A
TF|Technology|Easy|Python is a compiled language.|False
TF|Technology|Medium|Rust prevents memory safety issues at compile time.|True
TF|Technology|Hard|The first computer bug was an actual insect found in a relay.|True

// --- HISTORY ---
MC|History|Easy|In which year did World War II end?|1943|1944|1945|1946|C
MC|History|Easy|Who was the first person to walk on the Moon?|Buzz Aldrin|Yuri Gagarin|Neil Armstrong|John Glenn|C
MC|History|Medium|Which ancient wonder of the world still exists today?|Colossus of Rhodes|Great Pyramid of Giza|Hanging Gardens of Babylon|Lighthouse of Alexandria|B
MC|History|Hard|What year did the Berlin Wall fall?|1987|1988|1989|1990|C
TF|History|Easy|The Great Wall of China is visible from space.|False
TF|History|Medium|The Ottoman Empire lasted more than 600 years.|True

// --- GENERAL KNOWLEDGE ---
MC|General|Easy|How many sides does a hexagon have?|5|6|7|8|B
MC|General|Easy|What is the largest mammal on Earth?|Elephant|Blue Whale|Giraffe|Hippo|B
MC|General|Medium|What is the most spoken language in the world by native speakers?|English|Spanish|Hindi|Mandarin Chinese|D
MC|General|Hard|What is the Fibonacci sequence's 10th number?|34|55|89|144|B
TF|General|Easy|A group of crows is called a murder.|True
TF|General|Medium|Diamonds are made of carbon.|True
TF|General|Hard|The shortest war in history lasted 38 minutes.|True
"#;

    match fs::write(QUESTIONS_FILE, content) {
        Ok(_) => println!("  Created sample questions file: {}", QUESTIONS_FILE),
        Err(e) => println!("  Warning: Could not create questions file: {}", e),
    }
}

// =====================================================
// FILE — load all questions
// =====================================================
fn load_questions() -> Vec<Question> {
    if !Path::new(QUESTIONS_FILE).exists() {
        println!("  Questions file not found. Creating sample file...");
        create_sample_questions();
    }

    let content = match fs::read_to_string(QUESTIONS_FILE) {
        Ok(c) => c,
        Err(e) => {
            println!("  Error loading questions: {}", e);
            return Vec::new();
        }
    };

    // Parse each line once: blank lines and comments are ignored,
    // and lines that fail to parse are counted as skipped.
    let mut questions: Vec<Question> = Vec::new();
    let mut skipped = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        match Question::parse(trimmed) {
            Some(q) => questions.push(q),
            None => skipped += 1,
        }
    }

    println!("  Loaded {} questions.", questions.len());
    if skipped > 0 {
        println!("  Warning: {} invalid lines skipped.", skipped);
    }
    questions
}

// =====================================================
// FILE — save and load scores
// =====================================================
fn save_score(score: &Score) {
    let mut lines: Vec<String> = if Path::new(SCORES_FILE).exists() {
        fs::read_to_string(SCORES_FILE)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.to_string())
            .collect()
    } else {
        Vec::new()
    };

    lines.push(score.to_save_line());
    match fs::write(SCORES_FILE, lines.join("\n")) {
        Ok(_) => println!("Score saved successfully!"),
        Err(e) => {
            println!("Couldn't save score {}", e);
            return;
        }
    }
}

fn load_scores() -> Vec<Score> {
    if !Path::new(SCORES_FILE).exists() {
        return Vec::new();
    }

    fs::read_to_string(SCORES_FILE)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(Score::from_save_line)
        .collect()
}

// =====================================================
// HELPERS
// =====================================================

// Reads a non-empty line of text.
// '|' is the file delimiter, so it is stored as '~'.
// Names and categories turn '~' back into '|' only
// when they are displayed.
fn read_input(prompt: &str) -> String {
    loop {
        println!("{}", prompt);
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read");
        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed.replace('|', "~");
        }
        println!("  Input cannot be empty.");
    }
}

// Reads a usize, re-asking until the input parses.
// It does NOT check any range — the caller does that
// with a `match` (e.g. `0 => ..., n if n <= max => ...`).
fn read_usize(prompt: &str) -> usize {
    loop {
        println!("{}", prompt);
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read");
        match input.trim().parse::<usize>() {
            Ok(n) => return n,
            Err(_) => println!("  Please enter a valid whole number."),
        }
    }
}

// Returns true (and says so) if an answer took longer than the
// time limit. The clock is checked after the answer is entered,
// because terminal input can't be interrupted.
fn is_too_slow(timer: &Instant) -> bool {
    // Compare with decimals so 20.9s is not rounded down to 20s.
    let taken = timer.elapsed().as_secs_f64();
    if taken > QUESTION_TIME_LIMIT_SECS as f64 {
        println!(
            "  Too slow! You took {:.1}s (limit: {}s). This counts as wrong.",
            taken, QUESTION_TIME_LIMIT_SECS
        );
        true
    } else {
        false
    }
}

// Returns the sorted, de-duplicated list of categories.
fn get_categories(questions: &[Question]) -> Vec<String> {
    let mut cats: Vec<String> = questions.iter().map(|q| q.category.clone()).collect();
    cats.sort();
    cats.dedup();
    cats
}

// =====================================================
// QUIZ ENGINE
//
// Asks one question and returns whether it was answered
// correctly, plus a WrongAnswer record when it wasn't.
// `number` and `total` show "Question X/Y" in the header;
// `streak` is the current run of correct answers, used
// for the hot-streak message. An answer entered after
// QUESTION_TIME_LIMIT_SECS counts as wrong.
// =====================================================
fn ask_question(
    question: &Question,
    number: usize,
    total: usize,
    streak: u32,
) -> (bool, Option<WrongAnswer>) {
    // Show the hot-streak message on every even streak of 2 or
    // more, so it doesn't appear after every single question.
    if streak >= 2 && streak % 2 == 0 {
        println!("  *** HOT STREAK: {} correct in a row! ***", streak);
    }
    println!("\n  {}", "=".repeat(42));
    println!(
        "  Question {}/{} | {} | {} point(s) | {}s limit",
        number,
        total,
        question.difficulty,
        question.points(),
        QUESTION_TIME_LIMIT_SECS
    );
    println!("  {}", "=".repeat(42));
    println!("\n  {}\n", question.text);

    // The clock starts once the question is on screen.
    let question_timer = Instant::now();

    match &question.kind {
        QuestionKind::MultiChoice { options, answer } => {
            for option in options {
                println!("    {}", option);
            }
            println!();

            let player_answer: char = loop {
                println!("  Your answer (A/B/C/D):");
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("Failed");
                let ch = input.trim().to_uppercase();
                if let Some(c) = ch.chars().next() {
                    if "ABCD".contains(c) {
                        break c;
                    }
                }
                println!("  Please enter A, B, C, or D.");
            };

            let correct = player_answer == *answer && !is_too_slow(&question_timer);

            if correct {
                println!("  CORRECT! +{} point(s)", question.points());
                return (correct, None);
            } else {
                // Options are stored in A, B, C, D order, so the letter
                // gives the index. This keeps the full text, e.g. "C) Canberra".
                let correct_text = options[(*answer as u8 - b'A') as usize].clone();
                println!("  Wrong. The correct answer was: {}", correct_text);
                let wrong = WrongAnswer {
                    question_text: question.text.clone(),
                    player_answer: player_answer.to_string(),
                    correct_answer: correct_text,
                };
                return (false, Some(wrong));
            }
        }

        QuestionKind::TrueFalse { answer } => {
            println!("    T) True");
            println!("    F) False");
            println!();

            let player_answer: bool = loop {
                println!("  Your answer (T/F):");
                let mut input = String::new();
                io::stdin().read_line(&mut input).expect("Failed");
                match input.trim().to_uppercase().as_str() {
                    "T" | "TRUE" => break true,
                    "F" | "FALSE" => break false,
                    _ => println!("  Please enter T or F."),
                }
            };

            let correct = player_answer == *answer && !is_too_slow(&question_timer);

            if correct {
                println!("  CORRECT! +{} point(s)", question.points());
                return (correct, None);
            } else {
                println!(
                    "  Wrong. The correct answer was: {}",
                    if *answer { "True" } else { "False" }
                );
                let wrong = WrongAnswer {
                    question_text: question.text.clone(),
                    player_answer: (if player_answer { "True" } else { "False" }).to_string(),
                    correct_answer: (if *answer { "True" } else { "False" }).to_string(),
                };
                return (false, Some(wrong));
            }
        }
    }
}

// =====================================================
// QUIZ RUNNER
//
// Filters and shuffles the questions, asks them while
// tracking score, streaks and time, then shows the results,
// saves the score and offers a review of wrong answers.
// =====================================================
fn run_quiz(all_questions: &[Question], category: &str, difficulty: &str, name: &str) {
    let mut filtered: Vec<&Question> = all_questions
        .iter()
        .filter(|q| {
            let cat_match = category == "All" || q.category == category;
            let diff_match = difficulty == "All" || q.difficulty == difficulty;
            cat_match && diff_match
        })
        .collect();

    if filtered.is_empty() {
        println!("\n  No questions found for that combination. Try different settings.");
        return;
    }

    let mut rng = rand::rng();
    filtered.shuffle(&mut rng);

    println!("\n  {} question(s) available.", filtered.len());

    let count: usize = loop {
        match read_usize(&format!(
            "  How many do you want? (1 to {}):",
            filtered.len()
        )) {
            n if n >= 1 && n <= filtered.len() => break n,
            _ => println!("  Enter a number between 1 and {}.", filtered.len()),
        }
    };

    let quiz_questions = &filtered[..count];
    let total = quiz_questions.len();

    println!(
        "\n  Starting quiz: {} questions | {} | {}",
        total,
        category.replace('~', "|"),
        difficulty
    );
    println!("  The timer starts NOW. Good luck!\n");

    let timer = Instant::now();

    let mut correct_count: u32 = 0;
    let mut total_points: u32 = 0;
    let mut max_points: u32 = 0;
    let mut wrong_answers: Vec<WrongAnswer> = Vec::new();
    let mut streak: u32 = 0; // current run of correct answers
    let mut best_streak: u32 = 0; // longest run reached this quiz

    for (i, question) in quiz_questions.iter().enumerate() {
        max_points += question.points();

        // Pass the current streak so ask_question can display it
        let (correct, wrong_record) = ask_question(question, i + 1, total, streak);

        if correct {
            correct_count += 1;
            total_points += question.points();
            streak += 1;
            // Update best streak whenever the current one beats it
            if streak > best_streak {
                best_streak = streak;
            }
        } else {
            if let Some(w) = wrong_record {
                wrong_answers.push(w);
            }
            streak = 0; // Reset streak on a wrong answer
        }

        // Running score — show progress after every question
        println!(
            "  [ Score so far: {}/{} correct | {} pts ]",
            correct_count,
            i + 1,
            total_points
        );
    }

    let elapsed_secs = timer.elapsed().as_secs();

    let score = Score {
        name: name.to_string(),
        correct: correct_count,
        total: total as u32,
        points: total_points,
        max_points,
        time_secs: elapsed_secs,
        category: format!("{} / {}", category, difficulty),
        best_streak,
    };

    let minutes = elapsed_secs / 60;
    let seconds = elapsed_secs % 60;

    println!("\n  {}", "=".repeat(42));
    println!("              QUIZ COMPLETE!");
    println!("  {}", "=".repeat(42));
    println!("  Player      : {}", name.replace('~', "|"));
    println!(
        "  Category    : {} | {}",
        category.replace('~', "|"),
        difficulty
    );
    println!("  Score       : {}/{}", correct_count, total);
    println!("  Percentage  : {}%", score.percentage());
    println!("  Points      : {}/{}", total_points, max_points);
    println!("  Grade       : {}", score.grade());
    println!("  Best Streak : {}", best_streak);
    println!("  Time        : {}m {}s", minutes, seconds);
    println!("  {}", "=".repeat(42));

    let filled = score.percentage() as usize / 5;
    let bar = format!("[{}{}]", "#".repeat(filled), "-".repeat(20 - filled));
    println!("  Progress    : {} {}%", bar, score.percentage());
    println!("  {}\n", "=".repeat(42));

    save_score(&score);
    println!("  Score saved!");

    if !wrong_answers.is_empty() {
        println!("\n  You got {} question(s) wrong.", wrong_answers.len());
        println!("  Review wrong answers? (yes/no):");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed");
        if input.trim().to_lowercase().starts_with('y') {
            println!(
                "\n  {} WRONG ANSWERS REVIEW {}\n",
                "-".repeat(4),
                "-".repeat(4)
            );
            for (i, wa) in wrong_answers.iter().enumerate() {
                println!("  {}. {}", i + 1, wa.question_text);
                println!("     Your answer    : {}", wa.player_answer);
                println!("     Correct answer : {}", wa.correct_answer);
                println!();
            }
        }
    } else {
        println!("\n  Perfect score! No wrong answers!");
    }
}

// =====================================================
// MENU — quiz setup
//
// Asks for the player's name, category and difficulty,
// then starts the quiz.
// =====================================================
fn start_quiz_menu(questions: &[Question]) {
    if questions.is_empty() {
        println!("  No questions loaded. Check your questions.txt file.");
        return;
    }

    let name = read_input("\n  Enter your name:");

    let categories = get_categories(questions);

    println!("\n  {} Select Category {}", "-".repeat(3), "-".repeat(3));
    println!("  0. All Categories");
    for (i, cat) in categories.iter().enumerate() {
        println!("  {}. {}", i + 1, cat.replace('~', "|"));
    }

    let category: String = loop {
        match read_usize("  Choose:") {
            0 => break String::from("All"),
            n if n <= categories.len() => break categories[n - 1].clone(),
            _ => println!("  Invalid choice."),
        }
    };

    println!("\n  {} Select Difficulty {}", "-".repeat(3), "-".repeat(3));
    println!("  0. All Difficulties");
    println!("  1. Easy   (+1 point each)");
    println!("  2. Medium (+2 points each)");
    println!("  3. Hard   (+3 points each)");

    let difficulty: String = loop {
        match read_usize("  Choose:") {
            0 => break String::from("All"),
            1 => break String::from("Easy"),
            2 => break String::from("Medium"),
            3 => break String::from("Hard"),
            _ => println!("  Please enter 0, 1, 2, or 3."),
        }
    };

    run_quiz(questions, &category, &difficulty, &name);
}

// =====================================================
// MENU — high scores
//
// Shows summary stats (total games, average %, best
// player) and the top TOP_SCORES scores, with an option to
// clear all scores at the bottom.
// =====================================================
fn show_scores_menu() {
    let mut scores = load_scores();

    if scores.is_empty() {
        println!("\n  No scores recorded yet. Play a quiz first!");
        return;
    }

    // Sort by points descending, then percentage descending as tiebreaker
    scores.sort_by(|a, b| {
        b.points
            .cmp(&a.points)
            .then(b.percentage().cmp(&a.percentage()))
    });

    // Summary stats across all recorded games
    let total_games = scores.len() as u32;
    let total_pct: u32 = scores.iter().map(|s| s.percentage()).sum();
    let avg_pct: f64 = total_pct as f64 / total_games as f64; // f64 so the average keeps its decimals
    let best = &scores[0]; // Already sorted — first entry is highest score

    println!("\n  {}", "=".repeat(42));
    println!("            HIGH SCORES");
    println!("  {}", "=".repeat(42));
    println!("  Total Games  : {}", total_games);
    println!("  Average Score: {:.1}%", avg_pct); // one decimal place
    println!(
        "  Best Player  : {} ({} pts, {}%)",
        best.name.replace('~', "|"),
        best.points,
        best.percentage()
    );
    println!("  {}", "=".repeat(42));
    println!(
        "  {:<4} {:<15} {:<8} {:<8} {:<6} {:<6} {:<7} {}",
        "Rank", "Name", "Score", "Points", "%", "Grade", "Streak", "Category"
    );
    println!("  {}", "-".repeat(72));

    for (i, s) in scores.iter().enumerate().take(TOP_SCORES) {
        println!(
            "  {:<4} {:<15} {:<8} {:<8} {:<6} {:<6} {:<7} {}",
            format!("#{}", i + 1),
            s.name.replace('~', "|"),
            format!("{}/{}", s.correct, s.total),
            format!("{}/{}", s.points, s.max_points),
            format!("{}%", s.percentage()),
            s.grade(),
            s.best_streak,
            s.category.replace('~', "|")
        );
    }

    println!("  {}", "-".repeat(72));
    println!();

    // Let the player clear all saved scores
    println!("  Clear all scores? (yes/no):");
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed");

    if input.trim().to_lowercase().starts_with('y') {
        match fs::remove_file(SCORES_FILE) {
            Ok(_) => println!("  All scores cleared."),
            Err(e) => println!("  Could not clear scores: {}", e),
        }
    }
}

// =====================================================
// MENU — add a custom question
//
// Collects the fields, builds one line in the questions
// file format and appends it. The difficulty goes through
// capitalize() so "easy" is stored as "Easy", matching the
// quiz filter.
// =====================================================
fn add_question_menu() {
    println!("\n{} Add Custom Question {}", "-".repeat(3), "-".repeat(3));

    let q_type = loop {
        println!("  Question type:");
        println!("  1. Multiple Choice");
        println!("  2. True / False");
        match read_usize("  Choose:") {
            1 => break "MC",
            2 => break "TF",
            _ => println!("  Enter 1 or 2."),
        }
    };

    let category = read_input("  Category (e.g. Science, History):");

    // capitalize() normalizes whatever the user types
    let difficulty = loop {
        println!("  Difficulty — Easy, Medium, or Hard:");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed");
        let d = capitalize(input.trim()); // "easy" -> "Easy", "HARD" -> "Hard"
        match d.to_lowercase().as_str() {
            "easy" | "medium" | "hard" => break d,
            _ => println!("  Enter Easy, Medium, or Hard."),
        }
    };

    let question_text = read_input("  Question text:");

    let line = if q_type == "MC" {
        let a = read_input("  Option A:");
        let b = read_input("  Option B:");
        let c = read_input("  Option C:");
        let d = read_input("  Option D:");

        let answer = loop {
            println!("  Correct answer (A/B/C/D):");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed");
            let ch = input.trim().to_uppercase();
            if let Some(c) = ch.chars().next() {
                if "ABCD".contains(c) {
                    break c;
                }
            }
            println!("  Enter A, B, C, or D.");
        };

        format!(
            "MC|{}|{}|{}|{}|{}|{}|{}|{}",
            category, difficulty, question_text, a, b, c, d, answer
        )
    } else {
        let answer = loop {
            println!("  Correct answer (True/False):");
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("Failed");
            match input.trim().to_lowercase().as_str() {
                "true" => break "True",
                "false" => break "False",
                _ => println!("  Enter True or False."),
            }
        };

        format!(
            "TF|{}|{}|{}|{}",
            category, difficulty, question_text, answer
        )
    };

    let existing = fs::read_to_string(QUESTIONS_FILE).unwrap_or_default();
    let new_content = format!("{}\n{}", existing.trim_end(), line);

    match fs::write(QUESTIONS_FILE, new_content) {
        // main() reloads the questions after this returns
        Ok(_) => println!("\n  Question added! Reloading questions..."),
        Err(e) => println!("\n  Failed to save question: {}", e),
    }
}

// =====================================================
// MENU — browse all questions
//
// Lists the categories with question counts, then shows
// the questions in the chosen category (or all of them).
// =====================================================
fn browse_questions(questions: &[Question]) {
    let categories = get_categories(questions);

    println!("\n{} Browse Questions {}", "-".repeat(3), "-".repeat(3));
    println!("  0. All");
    for (i, cat) in categories.iter().enumerate() {
        let count = questions.iter().filter(|q| q.category == *cat).count();
        println!(
            "  {}. {} ({} questions)",
            i + 1,
            cat.replace('~', "|"),
            count
        );
    }

    let filter: Option<String> = loop {
        match read_usize("  Choose category (or 0 for all):") {
            0 => break None,
            n if n <= categories.len() => break Some(categories[n - 1].clone()),
            _ => println!("  Invalid choice."),
        }
    };

    let filtered: Vec<&Question> = questions
        .iter()
        .filter(|q| filter.as_ref().is_none_or(|cat| &q.category == cat))
        .collect();

    println!("\n  Showing {} question(s):\n", filtered.len());

    for (i, q) in filtered.iter().enumerate() {
        let kind_label = match q.kind {
            QuestionKind::MultiChoice { .. } => "MC",
            QuestionKind::TrueFalse { .. } => "TF",
        };
        println!(
            "  {}. [{}] [{}] [{}] {}",
            i + 1,
            kind_label,
            q.difficulty,
            q.category.replace('~', "|"),
            q.text
        );
    }
}

// =====================================================
// MAIN
//
// Loads the questions once, then runs the menu loop.
// `questions` is mutable so it can be reloaded from disk
// after a custom question is added, making the new
// question usable immediately.
// =====================================================
fn main() {
    println!("{}", "=".repeat(44));
    println!("               QUIZ APP                    ");
    println!("{}", "=".repeat(44));

    // `mut` because we reassign it after adding a question
    let mut questions = load_questions();

    loop {
        println!("\n{} MAIN MENU {}", "=".repeat(10), "=".repeat(10));
        println!("1. Start quiz");
        println!("2. Browse questions");
        println!("3. Add custom question");
        println!("4. High scores");
        println!("5. Exit");
        println!("{}", "=".repeat(31));

        match read_usize("Choose an option:") {
            1 => start_quiz_menu(&questions),
            2 => browse_questions(&questions),
            3 => {
                add_question_menu();
                // Reload questions from disk so the new one is immediately available
                questions = load_questions();
            }
            4 => show_scores_menu(),
            5 => {
                println!("\nThanks for playing. Goodbye!");
                break;
            }
            _ => println!("Invalid option. Please choose 1 to 5."),
        }
    }
}
