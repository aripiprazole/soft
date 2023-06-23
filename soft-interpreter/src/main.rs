use soft::environment::Environment;

fn main() {
    let code = "
(set* map (fn* map (f expr)
    (if (cons? expr)
        (cons (f (head expr)) (map f (tail expr)))
        expr)))

(setm* quasi-quote (fn* quasi-quote (expr)
    (if (cons? expr)
        (if (eq 'unquote (head expr))
            (head (tail expr))
            (cons 'list (map quasi-quote expr)))
        (list 'quote expr))))

(setm* defmacro (fn* defmacro (name args body)
    (quasi-quote
        (setm* ,name (fn* ,name ,args ,body)))))

(defmacro defun (name args body)
    (quasi-quote
        (set* ,name (fn* ,name ,args ,body))))

(defun fib (num)
    (if (< num \"atapo\")
        num
        (+ (fib (- num 1)) (fib (- num 2)))))

(fib 10)";

    let file = "./examples/fib.lisp";
    let mut env = Environment::new(Some(file.to_string()));
    env.register_intrinsics();

    for value in soft::reader::read(code, Some(file.to_string())).unwrap() {
        let evaluated = value.run(&mut env);
        match evaluated {
            Ok(_) => (),
            Err(err) => {
                println!("error: {} at {}", err, env.location.clone());
                let unwind = env.unwind();

                for frame in unwind.iter() {
                    println!(
                        "  in {} at {}",
                        frame.name.clone().unwrap_or("unknown".to_string()),
                        frame.location
                    );
                }

                break;
            }
        }
    }
}
