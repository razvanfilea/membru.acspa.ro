import {ActionIcon, Group, NumberInput, NumberInputHandlers} from "@mantine/core";
import React from "react";
import {Location} from "../../types/wrapper";

interface IParams {
    inputHandler: React.MutableRefObject<NumberInputHandlers | undefined>;
    isWeekend: boolean;
    gameLocation: Location;
    formProps: any;
}

export default function AdminHourInput({inputHandler, isWeekend, gameLocation, formProps}: IParams) {

    return <Group spacing={8} noWrap={true} align={'end'}>
        <NumberInput
            {...formProps}
            handlersRef={inputHandler}
            hideControls={true}
            placeholder="Ora"
            label="Ora"
            disabled={true}
            required={true}
            step={isWeekend ? gameLocation.weekend_reservation_duration : gameLocation.reservation_duration}
            min={isWeekend ? gameLocation.weekend_start_hour : gameLocation.start_hour}
            max={isWeekend ? (gameLocation.weekend_end_hour - gameLocation.weekend_reservation_duration)
                : (gameLocation.end_hour - gameLocation.reservation_duration)}
        />

        <ActionIcon size={36} variant="default"
                    onClick={() => inputHandler.current!.decrement()}>
            –
        </ActionIcon>

        <ActionIcon size={36} variant="default"
                    onClick={() => inputHandler.current!.increment()}>
            +
        </ActionIcon>
    </Group>
}
