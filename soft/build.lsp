(require 'stdlib)

(println (-> stdlib/args
             (map (fun [x] (str "Hello " x)))))