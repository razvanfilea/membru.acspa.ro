import {Group, Radio} from "@mantine/core";
import React, {useEffect} from "react";
import {Location} from "../../types/wrapper";

interface IParams {
    isWeekend: boolean;
    gameLocation: Location;
    formProps: any;
}

export default function AdminHourInput({isWeekend, gameLocation, formProps}: IParams) {

    const duration = isWeekend ? gameLocation.weekend_reservation_duration : gameLocation.reservation_duration;
    const start_hour = isWeekend ? gameLocation.weekend_start_hour : gameLocation.start_hour;
    const end_hour = isWeekend ? (gameLocation.weekend_end_hour - gameLocation.weekend_reservation_duration)
        : (gameLocation.end_hour - gameLocation.reservation_duration);

    let content: React.JSX.Element[] = []

    useEffect(() => {
        formProps.onChange((isWeekend ? gameLocation.weekend_start_hour : gameLocation.start_hour).toString())
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [gameLocation, isWeekend]);


    for (let hour = start_hour; hour <= end_hour; hour += duration) {
        content.push(<React.Fragment key={hour}>
            <Radio value={hour.toString()} label={hour} size={'md'}/>
        </React.Fragment>)
    }

    return <Radio.Group
        {...formProps}
        label={"Ora"}
        size={'lg'}
        withAsterisk>
        <Group gap='md' p={'sm'}>{content}</Group>
    </Radio.Group>
}
