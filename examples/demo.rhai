fn main() {
    let osc = processor(#{
        input_spec: [
            ["freq", "f32"],
            ["sample_rate", "f32"],
        ],
        output_spec: [
            ["output", "f32"],
        ],
        t: 0.0,
        process: |freq, sample_rate| {
            this.t += 1.0;
            let t2 =  this.t * freq * 6.283185307179586476925286766559; // 2 * pi
            let out = sin(t2 / sample_rate) * 0.1;
            return [
                out,
            ];
        },
    }).inputs([440.0, sample_rate()]);

    
    dac(osc.output(0));
    dac(osc.output(0));
    
    play();
    sleep(5);
    stop();
}
