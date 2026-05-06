// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract WorstCaseERC20 {
    // ERC20 State
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;
    uint256 public totalSupply;

    // Token metadata - returning constants to save gas
    string public constant name = "WorstCase";
    string public constant symbol = "WORST";
    uint8 public constant decimals = 18;

    constructor() {
        // Mint total supply to deployer
        totalSupply = 1_000_000_000 * 10 ** 18; // 1 billion tokens
        balanceOf[msg.sender] = totalSupply;

        // Set all mined storage keys to 1
        assembly {
            sstore(0x90aed6381df22c4793cff47b9de601bfb9b2453728a30c06c2467b8f853cb2d1, 1)
            sstore(0xf285f783535d70331ab55647af851d3b1b9602511d3170a063eb5f6009f61f97, 1)
            sstore(0xf846dde7b4f69edaa55b373f371ddba3a4737fe7374aadf4d7e8bf6516d8abb2, 1)
            sstore(0xf83455c07e01137577ca7a6b8ddffe68539d909032653e9517d84f95dcfc9bc7, 1)
            sstore(0xf83a3f2284ce9a0c47209657896e78674a0f65e35db449d59b5824adc17acf30, 1)
            sstore(0xf83ac496a1ef27e28ce3a43f4568df5486982984eeade7a721bf7e101dbdf18d, 1)
            sstore(0xf83ac44e59d76029037c08c7b04da715e9bd706028849a93fe1b201060e514ea, 1)
            sstore(0xf83ac4afccbd77c23a53a5b5154dbfd554a11a578c755d7041795e313b6b4647, 1)
            sstore(0xf83ac4afa609457763a21500bef1cbf7ef3a4bf52f7d97119720ea10994506fc, 1)
            sstore(0xf83ac4afb48b1737f3f3754c4626c3412530fca8a134c13a094269e1b539e4d9, 1)
            sstore(0xf83ac4afbeb3a2196fb9fa39819633ed866d97d02916d1ac7dca8a6e76099d21, 1)
            sstore(0xf83ac4afbe60eb89f126339d262bb4073deabc471a6905bbd07a008f56f059ab, 1)
        }
    }

    // Minimal ERC20 implementation
    function transfer(address to, uint256 amount) public returns (bool) {
        require(balanceOf[msg.sender] >= amount, "Insufficient balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }

    function approve(address spender, uint256 amount) public returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(
        address from,
        address to,
        uint256 amount
    ) public returns (bool) {
        require(balanceOf[from] >= amount, "Insufficient balance");
        require(
            allowance[from][msg.sender] >= amount,
            "Insufficient allowance"
        );

        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        allowance[from][msg.sender] -= amount;

        return true;
    }

    // Attack method - writes to the deepest storage slot
    function attack(uint256 value) external {
        assembly {
            sstore(0xf83ac4afbe60eb89f126339d262bb4073deabc471a6905bbd07a008f56f059ab, value)
        }
    }

    // Optional: getter to verify the deepest slot value
    function getDeepest() external view returns (uint256 value) {
        assembly {
            value := sload(0xf83ac4afbe60eb89f126339d262bb4073deabc471a6905bbd07a008f56f059ab)
        }
    }
}