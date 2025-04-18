const two_pi = 6.283185307179586476925286766559;

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
            this.t %= sample_rate;
            let out = sin(this.t * freq * global::two_pi / sample_rate) * 0.1;
            return [
                out,
            ];
        },
    }).set_inputs([440.0, sample_rate()]);

    
    dac(osc.output(0));
    dac(osc.output(0));
    
    play();
    sleep(5);
    stop();
}
